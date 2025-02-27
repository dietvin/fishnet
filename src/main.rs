mod error;
mod loader;
mod alignment;
mod refinement;
mod combined_data;


use loader::{pod5, bam};
use alignment::aligned_read::AlignedRead;

fn main() {
    let path: &str = "example_data/can_reads.pod5";
    let paths = vec![path.to_string()];
    let index = pod5::Pod5Index::from_files(&paths).unwrap();

    let path = "example_data/can_mappings.bam";
    let mut bam_file = bam::BamFileLazy::new(path).unwrap();

    for file in index.files() {
        if let Ok(file) = file {
            let mut file = file;
            for (_, read) in &mut file {
                println!("\n{}\n", read.read_id());
                println!("Loading corresponding bam read:");
                let bam_read = bam_file.get(read.read_id()).unwrap();

                println!("Aligning signal:");
                let mut aligned_read = AlignedRead::new(
                    read, 
                    &bam_read, 
                    false
                ).unwrap();

                aligned_read.align_query_to_signal().unwrap();
                aligned_read.align_reference_to_signal().unwrap();

                // println!("{:?}", aligned_read.query_to_signal());
                println!("Reference to signal:\n{:?}", aligned_read.reference_to_signal());
            }
        } else {
            println!("Failed to read File")
        }
    }
}