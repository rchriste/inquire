//! `inquire` is a library for building interactive prompts on terminals.
//!
//! It provides several different prompts in order to interactively ask the user
//! for information via the CLI. With `inquire`, you can use:
//!
//! - [`Text`] to get text input from the user, with _built-in auto-completion support_;
//! - [`DateSelect`]* to get a date input from the user, selected via an _interactive calendar_;
//! - [`Select`] to ask the user to select one option from a given list;
//! - [`MultiSelect`] to ask the user to select an arbitrary number of options from a given list;
//! - [`Confirm`] for simple yes/no confirmation prompts;
//! - [`CustomType`] for text prompts that you would like to parse to a custom type, such as numbers or UUIDs;
//! - [`Password`] for secretive text prompts.
//!
//! Check out the [GitHub repository](https://github.com/mikaelmello/inquire) to see demos of what you can do with `inquire`.
//!
//! # Features
//!
//! - Cross-platform, supporting UNIX and Windows terminals (thanks to [crossterm](https://crates.io/crates/crossterm));
//! - Several kinds of prompts to suit your needs;
//! - Standardized error handling (thanks to [thiserror](https://crates.io/crates/thiserror));
//! - Support for fine-grained configuration for each prompt type, allowing you to customize:
//!   - Default values;
//!   - Input validators and formatters;
//!   - Help messages;
//!   - Auto-completion for [`Text`] prompts;
//!   - Custom list filters for Select and [`MultiSelect`] prompts;
//!   - Custom parsers for [`Confirm`] and [`CustomType`] prompts;
//!   - and many others!
//!
//! \* Date-related features are available by enabling the `date` feature.
//!
//! # Simple Example
//!
//! ```rust no_run
//! use inquire::{max_length, Text};
//!
//! fn main() {
//!     let status = Text::new("What are you thinking about?")
//!         .with_validator(max_length!(140, "You're only allowed 140 characters."))
//!         .prompt();
//!     
//!     match status {
//!         Ok(status) => println!("Your status is being published..."),
//!         Err(err) => println!("Error while publishing your status: {}", err),
//!     }
//! }
//! ```
//!
//! [`Text`]: crate::Text
//! [`DateSelect`]: crate::DateSelect
//! [`Select`]: crate::Select
//! [`MultiSelect`]: crate::MultiSelect
//! [`Confirm`]: crate::Confirm
//! [`CustomType`]: crate::CustomType
//! [`Password`]: crate::Password

#![warn(missing_docs)]

pub mod config;
#[cfg(feature = "date")]
mod date_utils;
pub mod error;
pub mod formatter;
mod input;
pub mod option_answer;
pub mod parser;
mod prompts;
pub mod ui;
mod utils;
pub mod validator;

pub use crate::prompts::*;
