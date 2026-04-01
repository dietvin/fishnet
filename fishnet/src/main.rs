use crate::cli::init_cli;

mod cli;

fn main() {
    let command_line_input = init_cli().get_matches();

    match command_line_input.subcommand() {
        Some(("align", subcommand_args)) => {
            alignment::execute::execute(subcommand_args);
        }

        Some(("reformat", subcommand_args)) => {
            reformat::execute::execute(subcommand_args);
        }
        _ => {
            unreachable!("arg_required_else_help is set in init_cli")
        }
    }
}
