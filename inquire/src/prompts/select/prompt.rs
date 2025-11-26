use std::{cmp::Reverse, fmt::Display};

use crate::{
    error::InquireResult,
    formatter::OptionFormatter,
    input::{Input, InputActionResult},
    list_option::ListOption,
    prompts::action::Action,
    prompts::prompt::{ActionResult, Prompt},
    type_aliases::Scorer,
    ui::SelectBackend,
    utils::paginate,
    InquireError, Select,
};

use super::{action::SelectPromptAction, config::SelectConfig};

pub struct SelectPrompt<'a, T> {
    message: &'a str,
    config: SelectConfig,
    options: Vec<T>,
    string_options: Vec<String>,
    scored_options: Vec<usize>,
    help_message: Option<&'a str>,
    cursor_index: usize,
    input: Option<Input>,
    scorer: Scorer<'a, T>,
    formatter: OptionFormatter<'a, T>,
}

impl<'a, T> SelectPrompt<'a, T>
where
    T: Display,
{
    pub fn new(so: Select<'a, T>) -> InquireResult<Self> {
        if so.options.is_empty() {
            return Err(InquireError::InvalidConfiguration(
                "Available options can not be empty".into(),
            ));
        }

        if so.starting_cursor >= so.options.len() {
            return Err(InquireError::InvalidConfiguration(format!(
                "Starting cursor index {} is out-of-bounds for length {} of options",
                so.starting_cursor,
                &so.options.len()
            )));
        }

        let string_options = so.options.iter().map(T::to_string).collect();
        let scored_options = (0..so.options.len()).collect();

        let input = match so.filter_input_enabled {
            true => Some(Input::new_with(
                so.starting_filter_input.unwrap_or_default(),
            )),
            false => None,
        };

        Ok(Self {
            message: so.message,
            config: (&so).into(),
            options: so.options,
            string_options,
            scored_options,
            help_message: so.help_message,
            cursor_index: so.starting_cursor,
            input,
            scorer: so.scorer,
            formatter: so.formatter,
        })
    }

    fn move_cursor_up(&mut self, qty: usize, wrap: bool) -> ActionResult {
        let new_position = if wrap {
            let after_wrap = qty.saturating_sub(self.cursor_index);
            self.cursor_index
                .checked_sub(qty)
                .unwrap_or_else(|| self.scored_options.len().saturating_sub(after_wrap))
        } else {
            self.cursor_index.saturating_sub(qty)
        };

        self.update_cursor_position(new_position)
    }

    fn move_cursor_down(&mut self, qty: usize, wrap: bool) -> ActionResult {
        let mut new_position = self.cursor_index.saturating_add(qty);

        if new_position >= self.scored_options.len() {
            new_position = if self.scored_options.is_empty() {
                0
            } else if wrap {
                new_position % self.scored_options.len()
            } else {
                self.scored_options.len().saturating_sub(1)
            }
        }

        self.update_cursor_position(new_position)
    }

    fn update_cursor_position(&mut self, new_position: usize) -> ActionResult {
        if new_position != self.cursor_index {
            self.cursor_index = new_position;
            ActionResult::NeedsRedraw
        } else {
            ActionResult::Clean
        }
    }

    fn has_answer_highlighted(&mut self) -> bool {
        self.scored_options.get(self.cursor_index).is_some()
    }

    fn get_final_answer(&mut self) -> ListOption<T> {
        // should only be called after current cursor index is validated
        // on has_answer_highlighted

        let index = *self.scored_options.get(self.cursor_index).unwrap();
        let value = self.options.swap_remove(index);

        ListOption::new(index, value)
    }

    fn run_scorer(&mut self) {
        let content = match &self.input {
            Some(input) => input.content(),
            None => return,
        };

        let mut options = self
            .options
            .iter()
            .enumerate()
            .filter_map(|(i, opt)| {
                (self.scorer)(content, opt, self.string_options.get(i).unwrap(), i)
                    .map(|score| (i, score))
            })
            .collect::<Vec<(usize, i64)>>();

        options.sort_unstable_by_key(|(_idx, score)| Reverse(*score));

        let new_scored_options = options.iter().map(|(idx, _)| *idx).collect::<Vec<usize>>();

        if self.scored_options == new_scored_options {
            return;
        }

        self.scored_options = new_scored_options;

        if self.config.reset_cursor {
            let _ = self.update_cursor_position(0);
        } else if self.scored_options.len() <= self.cursor_index {
            let _ = self.update_cursor_position(self.scored_options.len().saturating_sub(1));
        }
    }
}

impl<'a, Backend, T> Prompt<Backend> for SelectPrompt<'a, T>
where
    Backend: SelectBackend,
    T: Display,
{
    type Config = SelectConfig;
    type InnerAction = SelectPromptAction;
    type Output = ListOption<T>;

    fn message(&self) -> &str {
        self.message
    }

    fn config(&self) -> &SelectConfig {
        &self.config
    }

    fn format_answer(&self, answer: &ListOption<T>) -> String {
        (self.formatter)(answer.as_ref())
    }

    fn setup(&mut self) -> InquireResult<()> {
        self.run_scorer();
        Ok(())
    }

    fn prompt(mut self, backend: &mut Backend) -> InquireResult<Self::Output> {
        // NOTE ABOUT OVERRIDING `Prompt::prompt`
        //
        // The base `Prompt::prompt` implementation (see `prompts/prompt.rs`) includes an
        // explicit guideline to only override it with a strong reason. This override
        // exists to implement an *adaptive page size* workaround for terminals where
        // wrapped multi-line options can cause the rendered list to exceed the visible
        // screen height and corrupt the UI.
        //
        // Why this cannot live in the base `Prompt::prompt` today:
        //
        // 1) **Prompt-specific knob**: Adaptive sizing needs to shrink
        //    `self.config.page_size`. The base prompt is generic over `Self::Config`
        //    and has no required interface to read/modify a "page size" field. Without
        //    adding new trait requirements or hooks, the base prompt can't safely
        //    mutate prompt-specific configuration.
        //
        // 2) **Preflight + abort**: The workaround must render into an in-memory frame,
        //    compare the *flush height* (max of last/current frame heights) against
        //    terminal height, and if oversized, abort without flushing and retry with a
        //    smaller page size. The base prompt previously had no "abort and retry"
        //    control flow, so we need to own the redraw path here.
        //
        // 3) **State correction**: After shrinking, we may need to clamp cursor/selection
        //    indices and re-paginate. This adjustment is prompt-specific; the base
        //    prompt cannot assume how to reconcile internal state for all prompt types.
        //
        // Future direction:
        // If we want to move adaptive sizing into the base prompt for broader coverage,
        // we should introduce a minimal opt-in trait/hook (e.g., "AdaptivePagePrompt")
        // that exposes:
        //   - current page size
        //   - ability to set a smaller page size
        //   - a hook to reconcile state after resize
        // and have the default loop perform preflight/abort/retry only for prompts that
        // implement that trait. Until such an abstraction exists, this local override
        // is the smallest safe, semver-friendly fix.
        //
        <Self as Prompt<Backend>>::setup(&mut self)?;

        let mut last_handle = ActionResult::NeedsRedraw;
        let final_answer = loop {
            if last_handle.needs_redraw() {
                self.redraw_with_adaptive_page_size(backend)?;
                last_handle = ActionResult::Clean;
            }

            let key = backend.read_key()?;
            let action = Action::from_key(key, <Self as Prompt<Backend>>::config(&self));

            if let Some(action) = action {
                last_handle = match action {
                    Action::Submit => {
                        if let Some(answer) =
                            <Self as Prompt<Backend>>::submit(&mut self)?
                        {
                            break answer;
                        }
                        ActionResult::NeedsRedraw
                    }
                    Action::Cancel => {
                        let pre_cancel_result =
                            <Self as Prompt<Backend>>::pre_cancel(&mut self)?;

                        if pre_cancel_result {
                            backend.frame_setup()?;
                            backend.render_canceled_prompt(
                                <Self as Prompt<Backend>>::message(&self),
                            )?;
                            backend.frame_finish(true)?;
                            return Err(InquireError::OperationCanceled);
                        }

                        ActionResult::NeedsRedraw
                    }
                    Action::Interrupt => return Err(InquireError::OperationInterrupted),
                    Action::Inner(inner_action) => {
                        <Self as Prompt<Backend>>::handle(&mut self, inner_action)?
                    }
                };
            }
        };

        let formatted = <Self as Prompt<Backend>>::format_answer(&self, &final_answer);

        backend.frame_setup()?;
        backend.render_prompt_with_answer(
            <Self as Prompt<Backend>>::message(&self),
            &formatted,
        )?;
        backend.frame_finish(true)?;

        Ok(final_answer)
    }

    fn submit(&mut self) -> InquireResult<Option<ListOption<T>>> {
        let answer = match self.has_answer_highlighted() {
            true => Some(self.get_final_answer()),
            false => None,
        };

        Ok(answer)
    }

    fn handle(&mut self, action: SelectPromptAction) -> InquireResult<ActionResult> {
        let result = match action {
            SelectPromptAction::MoveUp => self.move_cursor_up(1, true),
            SelectPromptAction::MoveDown => self.move_cursor_down(1, true),
            SelectPromptAction::PageUp => self.move_cursor_up(self.config.page_size, false),
            SelectPromptAction::PageDown => self.move_cursor_down(self.config.page_size, false),
            SelectPromptAction::MoveToStart => self.move_cursor_up(usize::MAX, false),
            SelectPromptAction::MoveToEnd => self.move_cursor_down(usize::MAX, false),

            SelectPromptAction::FilterInput(input_action) => match self.input.as_mut() {
                Some(input) => {
                    let result = input.handle(input_action);

                    if let InputActionResult::ContentChanged = result {
                        self.run_scorer();
                    }

                    result.into()
                }
                None => ActionResult::Clean,
            },
        };

        Ok(result)
    }

    fn render(&self, backend: &mut Backend) -> InquireResult<()> {
        let prompt = &self.message;

        backend.render_select_prompt(prompt, self.input.as_ref())?;

        let choices = self
            .scored_options
            .iter()
            .cloned()
            .map(|i| ListOption::new(i, self.options.get(i).unwrap()))
            .collect::<Vec<ListOption<&T>>>();

        let page = paginate(self.config.page_size, &choices, Some(self.cursor_index));

        backend.render_options(page)?;

        if let Some(help_message) = self.help_message {
            backend.render_help_message(help_message)?;
        }

        Ok(())
    }
}

impl<'a, T> SelectPrompt<'a, T>
where
    T: Display,
{
    fn redraw_with_adaptive_page_size<Backend>(&mut self, backend: &mut Backend) -> InquireResult<()>
    where
        Backend: SelectBackend,
    {
        let mut page_size = self.config.page_size.max(1);

        loop {
            backend.frame_setup()?;
            self.render(backend)?;

            // Use the height that would actually be flushed (max of last/current)
            // to avoid scrolling when clearing a previously taller frame.
            let frame_h = backend.current_flush_height().unwrap_or(0);
            let term_h = backend.current_terminal_height().unwrap_or(u16::MAX);

            if frame_h <= term_h || page_size <= 1 {
                backend.frame_finish(false)?;
                break;
            }

            // Oversized render: abort without flushing and reduce page size.
            backend.frame_abort()?;

            let mut new_size = (page_size as u32)
                .saturating_mul(term_h.max(1) as u32)
                .checked_div(frame_h.max(1) as u32)
                .unwrap_or(1) as usize;

            if new_size >= page_size {
                new_size = page_size.saturating_sub(1).max(1);
            }

            page_size = new_size;
            self.config.page_size = page_size;
            // Ensure cursor stays within bounds after size change.
            let _ = self.update_cursor_position(self.cursor_index.min(self.scored_options.len().saturating_sub(1)));
        }

        Ok(())
    }
}
