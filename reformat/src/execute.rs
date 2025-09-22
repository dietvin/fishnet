pub mod init_cli;
pub(crate) mod config;
pub(crate) mod output;
mod execute_single_threaded;
mod execute_multi_threaded;

use clap::ArgMatches;
use console::style;

use crate::execute::{config::ConfigReformat, execute_multi_threaded::run_reformat_multi_threaded, execute_single_threaded::run_reformat_single_threaded};



pub fn execute(input_args: &ArgMatches) {
    let config = match ConfigReformat::from_argmatches(input_args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Failed to parse input data: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    };

    if config.n_threads() <= 1 {
        if let Err(e) = run_reformat_single_threaded(config) {
            eprintln!(
                "Failed to reformat the alignment: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    } else {
        if let Err(e) = run_reformat_multi_threaded(config) {
            eprintln!(
                "Failed to reformat the alignment: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}