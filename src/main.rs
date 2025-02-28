mod error;
mod loader;
mod alignment;
mod refinement;
mod combined_data;


use loader::{pod5, bam};
use alignment::aligned_read::AlignedRead;
use ::pod5::polars_arrow::bitmap::aligned::AlignedBitmapSlice;

fn main() {
    let path: &str = "example_data/can_reads.pod5";
    let paths = vec![path.to_string()];
    let index = pod5::Pod5Index::from_files(&paths).unwrap();

    let path = "example_data/can_mappings.bam";
    let mut bam_file = bam::BamFileLazy::new(path).unwrap();

    for file in index.files() {
        if let Ok(file) = file {
            let mut file = file;
            let mut pod5_read = file.get_mut("6e37823a-9398-4be8-b111-65cab029f4e0").unwrap();
            let bam_read = bam_file.get("6e37823a-9398-4be8-b111-65cab029f4e0").unwrap();
            let mut aligned_read = AlignedRead::new(pod5_read, &bam_read, false).unwrap();
            
            aligned_read.align_query_to_signal();
            aligned_read.align_reference_to_signal();

            println!("{:?}", aligned_read.reference_to_signal());
        //     for (_, read) in &mut file {
        //         println!("\n{}\n", read.read_id());
        //         println!("Loading corresponding bam read:");
        //         let bam_read = bam_file.get(read.read_id()).unwrap();

        //         println!("Aligning signal:");
        //         let mut aligned_read = AlignedRead::new(
        //             read, 
        //             &bam_read, 
        //             false
        //         ).unwrap();

        //         aligned_read.align_query_to_signal().unwrap();
        //         aligned_read.align_reference_to_signal().unwrap();

        //         // println!("{:?}", aligned_read.query_to_signal());
        //         println!("Reference to signal:\n{:?}", aligned_read.reference_to_signal());
        //     }
        // } else {
        //     println!("Failed to read File")
        // }
        }
    }
}