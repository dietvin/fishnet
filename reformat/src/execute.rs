use clap::ArgMatches;
use console::style;

use crate::execute::config::ConfigReformat;

pub mod init_cli;
pub(crate) mod config;

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

    println!("{:?}",config);
}