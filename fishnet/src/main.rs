use console::style;
use crate::{cli::init_cli};
pub mod cli;

fn main() {
    let command_line_input = init_cli().get_matches();

    match command_line_input.subcommand() {
        Some(("align", subcommand_args)) => {
            alignment::execute::execute(subcommand_args);
        }

        Some(("reformat", subcommand_args)) => {
            reformat::execute::execute(subcommand_args);
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
            eprintln!(
                "{}", format!("{}", style("No subcommand provided").red())
            )
        }
    }
}
