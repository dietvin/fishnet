pub mod cli;
pub mod core;
pub mod error;
pub mod logger;

use cli::execute_input::execute;

fn main() {
    execute();
}