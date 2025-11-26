use inquire::Select;
use std::fmt;

// Increase the number of options rendered on screen at once.
// Keeping this high helps reproduce issues when items wrap and total height exceeds the terminal.
const PAGE_SIZE: usize = 50;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    ManyLongItems,
    MixedLongAndShortItems,
}

impl Scenario {
    fn label(self) -> &'static str {
        match self {
            Scenario::ManyLongItems => "Scenario 1: many long items (all wrap, list taller than terminal)",
            Scenario::MixedLongAndShortItems => {
                "Scenario 2: many items, only a few are very long (mixed heights)"
            }
        }
    }
}

impl fmt::Display for Scenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

fn main() {
    let scenarios = vec![Scenario::ManyLongItems, Scenario::MixedLongAndShortItems];

    println!(
        "This example is meant to reproduce select/list rendering issues when options wrap \
over multiple lines and the list is taller than the terminal.\n\
Tip: shrink your terminal height to ~10-15 lines to make the bug obvious.\n"
    );

    let scenario = Select::new(
        "Select a repro scenario",
        scenarios.clone(),
    )
    .prompt();

    let scenario = match scenario {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Prompt aborted: {err}");
            return;
        }
    };

    match scenario {
        Scenario::ManyLongItems => run_many_long_items(),
        Scenario::MixedLongAndShortItems => run_mixed_items(),
    }
}

fn run_many_long_items() {
    // Lots of items, each intentionally long enough to wrap in most terminals.
    // Use a large number so the list is generally taller than the terminal.
    let options: Vec<String> = (1..=220)
        .map(|i| long_item(i, 3))
        .collect();

    let ans = Select::new(
        "Scenario 1: choose an item",
        options,
    )
    .with_page_size(PAGE_SIZE)
    .prompt();

    match ans {
        Ok(choice) => println!("You chose: {choice}"),
        Err(err) => eprintln!("Prompt aborted: {err}"),
    }
}

fn run_mixed_items() {
    // Many short items, with a few very long ones scattered that wrap to multiple lines.
    // Use a large number so the list is generally taller than the terminal.
    let mut options: Vec<String> = (1..=200)
        .map(|i| format!("item {i}"))
        .collect();

    // Insert long items at a few positions (spread out across the list).
    for (idx, i) in [
        (5usize, 101u32),
        (30, 102),
        (75, 103),
        (120, 104),
        (170, 105),
    ] {
        if idx < options.len() {
            options[idx] = long_item(i, 4);
        }
    }

    let ans = Select::new(
        "Scenario 2: choose an item",
        options,
    )
    .with_page_size(PAGE_SIZE)
    .prompt();

    match ans {
        Ok(choice) => println!("You chose: {choice}"),
        Err(err) => eprintln!("Prompt aborted: {err}"),
    }
}

fn long_item(i: u32, repeat: usize) -> String {
    // Builds a long string with spaces to encourage wrapping.
    let base = format!(
        "very long option #{i}: The quick brown fox jumps over the lazy dog — \
this chunk is here to exceed terminal width"
    );
    std::iter::repeat(base)
        .take(repeat)
        .collect::<Vec<_>>()
        .join(" | ")
}


