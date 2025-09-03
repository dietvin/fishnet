use clap::Command;

pub fn init_plot() -> Command {
    let command = Command::new("plot")
    .about("Visualize alignments");
    command
}