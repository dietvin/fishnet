use clap::ArgMatches;
use console::style;

use crate::execute::{config::Config, dispatch_pipeline::dispatch};

pub mod cli;
pub mod config;
mod dispatch_pipeline;
mod pipeline;

/// Entry point for the `align` sub-command.
///
/// Parses the raw [`ArgMatches`] produced by `clap` into a strongly-typed
/// [`Config`], then hands execution off to the dispatch layer which selects
/// the correct monomorphised pipeline at runtime.
///
/// # Errors
/// If [`Config::from_argmatches`] fails (e.g. a required argument is missing
/// or a value is out of range), the error is printed to `stderr` and the 
/// process exits with code `1`.
pub fn execute(args: &ArgMatches) {
    let config = match Config::from_argmatches(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "Failed to parse the command line input: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    };

    dispatch(config);

    std::process::exit(0);
}