mod error;
mod refinement;
mod loader;
mod combined_data;
use loader::{pod5};

fn main() {
    let path: &str = "example_data/can_reads.pod5";
    let paths = vec![path.to_string()];
    let index = pod5::Pod5Index::from_files(&paths).unwrap();

    println!("{:?}, {:?}", index.num_files(), index.num_loaded_files());

    for file in index.iter_files() {
        let (filename, file) = file.unwrap();
        println!("{filename}");
        for (_, read) in file.into_iter() {
            println!("{}, {}, {:?}, {:?}", 
                read.read_id(), 
                read.num_samples(), 
                read.calibration_offset(), 
                read.calibration_scale()
            );
        }
    }

    println!("{:?}, {:?}", index.num_files(), index.num_loaded_files());
}