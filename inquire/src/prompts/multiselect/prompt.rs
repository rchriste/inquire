use std::{cmp::Reverse, collections::BTreeSet, fmt::Display};

use crate::{
    error::InquireResult,
    formatter::MultiOptionFormatter,
    input::{Input, InputActionResult},
    list_option::ListOption,
    prompts::action::Action,
    prompts::prompt::{ActionResult, Prompt},
    type_aliases::Scorer,
    ui::MultiSelectBackend,
    utils::paginate,
    validator::{ErrorMessage, MultiOptionValidator, Validation},
    InquireError, MultiSelect,
};

use super::{action::MultiSelectPromptAction, config::MultiSelectConfig};

pub struct MultiSelectPrompt<'a, T> {
    message: &'a str,
    config: MultiSelectConfig,
    options: Vec<T>,
    string_options: Vec<String>,
    help_message: Option<&'a str>,
    cursor_index: usize,
    checked: BTreeSet<usize>,
    input: Option<Input>,
    scored_options: Vec<usize>,
    scorer: Scorer<'a, T>,
    formatter: MultiOptionFormatter<'a, T>,
    validator: Option<Box<dyn MultiOptionValidator<T>>>,
    error: Option<ErrorMessage>,
}

impl<'a, T> MultiSelectPrompt<'a, T>
where
    T: Display,
{
    pub fn new(mso: MultiSelect<'a, T>) -> InquireResult<Self> {
        if mso.options.is_empty() {
            return Err(InquireError::InvalidConfiguration(
                "Available options can not be empty".into(),
            ));
        }
        if let Some(default) = &mso.default {
            for i in default {
                if i >= &mso.options.len() {
                    return Err(InquireError::InvalidConfiguration(format!(
                        "Index {} is out-of-bounds for length {} of options",
                        i,
                        &mso.options.len()
                    )));
                }
            }
        }

        let string_options = mso.options.iter().map(T::to_string).collect();
        let scored_options = (0..mso.options.len()).collect();
        let checked_options = mso
            .default
            .as_ref()
            .map(|d| {
                d.iter()
                    .cloned()
                    .filter(|i| *i < mso.options.len())
                    .collect()
            })
            .unwrap_or_default();

        let input = match mso.filter_input_enabled {
            true => Some(Input::new_with(
                mso.starting_filter_input.unwrap_or_default(),
            )),
            false => None,
        };

        Ok(Self {
            message: mso.message,
            config: (&mso).into(),
            options: mso.options,
            string_options,
            scored_options,
            help_message: mso.help_message,
            cursor_index: mso.starting_cursor,
            input,
            scorer: mso.scorer,
            formatter: mso.formatter,
            validator: mso.validator,
            error: None,
            checked: checked_options,
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

    fn toggle_cursor_selection(&mut self) -> ActionResult {
        let idx = match self.scored_options.get(self.cursor_index) {
            Some(val) => val,
            None => return ActionResult::Clean,
        };

        if self.checked.contains(idx) {
            self.checked.remove(idx);
        } else {
            self.checked.insert(*idx);
        }

        ActionResult::NeedsRedraw
    }

    fn clear_input_if_needed(&mut self, action: MultiSelectPromptAction) -> ActionResult {
        if self.config.keep_filter {
            return ActionResult::Clean;
        }

        let input_ref = match &mut self.input {
            Some(input) => input,
            None => return ActionResult::Clean,
        };

        if input_ref.is_empty() {
            return ActionResult::Clean;
        }

        match action {
            MultiSelectPromptAction::ToggleCurrentOption
            | MultiSelectPromptAction::SelectAll
            | MultiSelectPromptAction::ClearSelections => {
                input_ref.clear();
                self.run_scorer();
                ActionResult::NeedsRedraw
            }
            _ => ActionResult::Clean,
        }
    }

    fn validate_current_answer(&self) -> InquireResult<Validation> {
        if let Some(validator) = &self.validator {
            let selected_options = self
                .options
                .iter()
                .enumerate()
                .filter_map(|(idx, opt)| match &self.checked.contains(&idx) {
                    true => Some(ListOption::new(idx, opt)),
                    false => None,
                })
                .collect::<Vec<_>>();

            let res = validator.validate(&selected_options)?;
            Ok(res)
        } else {
            Ok(Validation::Valid)
        }
    }

    fn get_final_answer(&mut self) -> Vec<ListOption<T>> {
        let mut answer = vec![];

        // by iterating in descending order, we can safely
        // swap remove because the elements to the right
        // that we did not remove will not matter anymore.
        for index in self.checked.iter().rev() {
            let index = *index;
            let value = self.options.swap_remove(index);
            let lo = ListOption::new(index, value);
            answer.push(lo);
        }
        answer.reverse();

        answer
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

impl<'a, Backend, T> Prompt<Backend> for MultiSelectPrompt<'a, T>
where
    Backend: MultiSelectBackend,
    T: Display,
{
    type Config = MultiSelectConfig;
    type InnerAction = MultiSelectPromptAction;
    type Output = Vec<ListOption<T>>;

    fn message(&self) -> &str {
        self.message
    }

    fn config(&self) -> &MultiSelectConfig {
        &self.config
    }

    fn format_answer(&self, answer: &Vec<ListOption<T>>) -> String {
        let refs: Vec<ListOption<&T>> = answer.iter().map(ListOption::as_ref).collect();
        (self.formatter)(&refs)
    }

    fn setup(&mut self) -> InquireResult<()> {
        self.run_scorer();
        Ok(())
    }

    fn prompt(mut self, backend: &mut Backend) -> InquireResult<Self::Output> {
        // NOTE ABOUT OVERRIDING `Prompt::prompt`
        //
        // See the analogous comment in `select/prompt.rs`. In short: we override the
        // base prompt loop here to implement adaptive page sizing for multi-line/wrapping
        // option lists without changing the public `Prompt` trait.
        //
        // The default `Prompt::prompt` loop always flushes a frame immediately after
        // `render(&self, ...)` with no chance to abort/retry, and it cannot mutate
        // `self.config.page_size` because `Self::Config` is opaque at the trait level.
        // MultiSelect also has prompt-specific state to reconcile (checked set, cursor)
        // after resizing. Therefore, the redraw path is owned here via
        // `redraw_with_adaptive_page_size`.
        //
        // Future direction: add a minimal opt-in adaptive-sizing trait/hook to the base
        // prompt so list-style prompts (Select/MultiSelect/Text suggestions, etc.) can
        // share this behavior without overrides.
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

    fn submit(&mut self) -> InquireResult<Option<Vec<ListOption<T>>>> {
        let answer = match self.validate_current_answer()? {
            Validation::Valid => Some(self.get_final_answer()),
            Validation::Invalid(msg) => {
                self.error = Some(msg);
                None
            }
        };

        Ok(answer)
    }

    fn handle(&mut self, action: MultiSelectPromptAction) -> InquireResult<ActionResult> {
        let result = match action {
            MultiSelectPromptAction::MoveUp => self.move_cursor_up(1, true),
            MultiSelectPromptAction::MoveDown => self.move_cursor_down(1, true),
            MultiSelectPromptAction::PageUp => self.move_cursor_up(self.config.page_size, false),
            MultiSelectPromptAction::PageDown => {
                self.move_cursor_down(self.config.page_size, false)
            }
            MultiSelectPromptAction::MoveToStart => self.move_cursor_up(usize::MAX, false),
            MultiSelectPromptAction::MoveToEnd => self.move_cursor_down(usize::MAX, false),
            MultiSelectPromptAction::ToggleCurrentOption => self.toggle_cursor_selection(),
            MultiSelectPromptAction::SelectAll => {
                self.checked.clear();
                for idx in &self.scored_options {
                    self.checked.insert(*idx);
                }
                ActionResult::NeedsRedraw
            }
            MultiSelectPromptAction::ClearSelections => {
                self.checked.clear();
                ActionResult::NeedsRedraw
            }
            MultiSelectPromptAction::FilterInput(input_action) => match self.input.as_mut() {
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

        let result = self.clear_input_if_needed(action).merge(result);

        Ok(result)
    }

    fn render(&self, backend: &mut Backend) -> InquireResult<()> {
        let prompt = &self.message;

        if let Some(err) = &self.error {
            backend.render_error_message(err)?;
        }

        backend.render_multiselect_prompt(prompt, self.input.as_ref())?;

        let choices = self
            .scored_options
            .iter()
            .cloned()
            .map(|i| ListOption::new(i, self.options.get(i).unwrap()))
            .collect::<Vec<ListOption<&T>>>();

        let page = paginate(self.config.page_size, &choices, Some(self.cursor_index));

        backend.render_options(page, &self.checked)?;

        if let Some(help_message) = self.help_message {
            backend.render_help_message(help_message)?;
        }

        Ok(())
    }
}

impl<'a, T> MultiSelectPrompt<'a, T>
where
    T: Display,
{
    fn redraw_with_adaptive_page_size<Backend>(
        &mut self,
        backend: &mut Backend,
    ) -> InquireResult<()>
    where
        Backend: MultiSelectBackend,
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
            let _ = self.update_cursor_position(
                self.cursor_index
                    .min(self.scored_options.len().saturating_sub(1)),
            );
        }

        Ok(())
    }
}
