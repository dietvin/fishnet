#[macro_use]
extern crate clap;

use alignment::execute::config::ConfigAlign;
use console::style;
use alignment::execute::execute;
use crate::{cli::init_cli};
pub mod cli;

fn main() {
    let command_line_input = init_cli().get_matches();

    match command_line_input.subcommand() {
        Some(("align", subcommand_args)) => {
            execute(subcommand_args);
        }

        Some(("reformat", subcommand_args)) => {
            println!("{:#?}", subcommand_args.try_get_one::<Vec<String>>("position-wise-stats"));
            println!("{:#?}", subcommand_args.try_get_one::<usize>("interpolate"));
        }
        // }
        // Some(("plot", subcommand_args)) => {
        //     let config = match ConfigPlot::from_cli(subcommand_args) {
        //         Ok(c) => c,
        //         Err(e) => {
        //             println!(
        //                 "Failed to parse input data: {}",
        //                 format!("{}", style(e).red())
        //             );
        //             std::process::exit(1);
        //         }
        //     };
        // }
        _ => {
            println!("No subcommand provided")
        }
    }
}
