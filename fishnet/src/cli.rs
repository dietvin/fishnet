use alignment::execute::init_cli::init_align;
use clap::{Command, crate_version};
use console::style;
use reformat::execute::init_cli::init_reformat;

pub fn init_cli() -> Command {
    
    let matches = Command::new("fishnet")
        .version(crate_version!())
        .author("Vincent Dietrich")
        .about(format!("{}", style("Fishnet - Signal-to-sequence alignment and processing").bold().green()))
        .arg_required_else_help(true)
        
        .subcommand(
            init_align()
        )
        
        .subcommand(
            init_reformat()
        );

    matches
}