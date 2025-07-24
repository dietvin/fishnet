pub mod cli;
pub mod core;
pub mod error;
pub mod logger;

use cli::execute::execute;

fn main() {
    execute();
}