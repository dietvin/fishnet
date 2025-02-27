mod error;
mod loader;
mod alignment;
mod refinement;
mod combined_data;


use loader::{pod5, bam};

fn main() {
    let path: &str = "example_data/can_reads.pod5";
    let paths = vec![path.to_string()];
    let index = pod5::Pod5Index::from_files(&paths).unwrap();

    let path = "example_data/can_mappings.bam";
    let mut bam_file = bam::BamFileLazy::new(path).unwrap();

    for file in index.files() {
        if let Ok(file) = file {
            for (_, read) in &file {
                println!("{}", read.read_id());
                println!("{}", read.signal().len());
                println!("{}", read.num_samples());
                println!("{:?}", read.calibration_offset());
                println!("{:?}", read.calibration_scale());

                println!("Loading corresponding bam read:");
                let bam_read = bam_file.get(read.read_id()).unwrap();
                println!("{}", bam_read.is_mapped());
                println!("{}\n", bam_read.stride());



            }
        } else {
            println!("Failed to read File")
        }
    }
}