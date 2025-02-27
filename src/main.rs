mod error;
mod loader;
mod alignment;
mod refinement;
mod combined_data;


use loader::{pod5};

fn main() {
    let path: &str = "example_data/can_reads.pod5";
    let paths = vec![path.to_string()];
    let index = pod5::Pod5Index::from_files(&paths).unwrap();

    for file in index.files() {
        if let Ok(file) = file {
            for (_, read) in &file {
                println!("{}", read.read_id());
                println!("{}", read.signal().len());
                println!("{}", read.num_samples());
                println!("{:?}", read.calibration_offset());
                println!("{:?}\n", read.calibration_scale());
            }
        } else {
            println!("Failed to read File")
        }
    }
}