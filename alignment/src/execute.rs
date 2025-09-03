/*!
 * Main entry point to the alignment process.
 */

pub mod config;
pub mod execute_multi_threaded;
pub mod execute_single_threaded;
pub mod output;
use clap::ArgMatches;
use console::style;
use execute_single_threaded::run_alignment_single_threaded;
use execute_multi_threaded::run_alignment_multi_threaded;

use crate::execute::config::ConfigAlign;

/// Entry point to the signal to sequence alignment.
/// 
/// Parses the given command, transforms it into a valid configuration and
/// executes it single- or multithreaded.
pub fn execute(input_args: &ArgMatches) {
    let config = match ConfigAlign::from_argmatches(input_args) {
        Ok(c) => c,
        Err(e) => {
            println!(
                "Failed to parse input data: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    };

    if config.n_threads() <= 1 {
        if let Err(e) = run_alignment_single_threaded(config) {
            println!(
                "Failed to perform alignment: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    } else {
        if let Err(e) = run_alignment_multi_threaded(config) {
            println!(
                "Failed to perform alignment: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}


