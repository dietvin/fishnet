pub(crate) mod init_alignment;
pub(crate) mod init_reformat;
pub(crate) mod init_plot;

use clap::{Command, crate_version};
use console::style;

use crate::cli::{init_alignment::init_align, init_plot::init_plot, init_reformat::init_reformat};

pub fn init_cli() -> Command {
    
    let matches = Command::new("fishnet")
        .version(crate_version!())
        .author("Vincent Dietrich")
        .about(format!("{}", style("Fishnet - Signal-to-sequence processing!").bold().green()))
        
        .subcommand(
            init_align()
        )
        
        .subcommand(
            init_reformat()
        )
        
        .subcommand(
            init_plot()
        );

    matches
}