mod error;
mod loader;
mod alignment;
mod refinement;
mod combined_data;


use loader::{pod5, bam};
use alignment::aligned_read::AlignedRead;
use refinement::signal_map_refiner::{settings::RefineSettings, SigMapRefiner};

fn main() {
    let path: &str = "example_data/can_reads.pod5";
    let paths = vec![path.to_string()];
    let index = pod5::Pod5Index::from_files(&paths).unwrap();

    let path = "example_data/can_mappings.bam";
    let mut bam_file = bam::BamFileLazy::new(path).unwrap();

    for file in index.files() {
        if let Ok(file) = file {
            let mut file = file;
            // let mut pod5_read = file.get_mut("6e37823a-9398-4be8-b111-65cab029f4e0").unwrap();
            // let bam_read = bam_file.get("6e37823a-9398-4be8-b111-65cab029f4e0").unwrap();

            for (read_id, read) in &mut file {
                println!("#### {} ####", read_id);

                let bam_read = bam_file.get(read.read_id()).unwrap();

                let mut aligned_read = AlignedRead::new(
                    read, 
                    &bam_read, 
                    false
                ).unwrap();
                
                aligned_read.align_query_to_signal().unwrap();
                aligned_read.align_reference_to_signal().unwrap();
    
                let settings: RefineSettings = RefineSettings::default();
                let mut sig_map_refiner = SigMapRefiner::new(
                    "example_data/levels.txt", 
                    &aligned_read, 
                    settings
                ).unwrap();
    
                sig_map_refiner.start().unwrap();
    
                let refined_query_to_sig = sig_map_refiner.refined_query_to_sig().unwrap();
    
                println!("{:?}", refined_query_to_sig);
            }
        }
    }
}