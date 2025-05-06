pub mod execute_multi_threaded;
pub mod execute_single_threaded;

use execute_single_threaded::run_alignment_single_threaded;
use execute_multi_threaded::run_alignment_multi_threaded;
use crate::cli::parse::{args_to_input::Config, init_cli::parse_command_line};

pub fn execute() {
    let command_line_input = parse_command_line();

    let input_data = match Config::from_argmatches(command_line_input) {
        Ok(input) => input,
        Err(e) => {
            println!("Failed to parse input data: {e}");
            std::process::exit(1);
        }
    };

    if input_data.n_threads() <= 1 {
        match run_alignment_single_threaded(input_data) {
            Ok(_) => println!("Finished sucessfully"),
            Err(e) => {
                println!("Failed to perform alignment: {e}");
                std::process::exit(1);
            }
        }
    } else {
        match run_alignment_multi_threaded(input_data) {
            Ok(_) => println!("Finished sucessfully"),
            Err(e) => {
                println!("Failed to perform alignment: {e}");
                std::process::exit(1);
            }
        }
    }

}


