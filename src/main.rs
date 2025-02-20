mod refinement;
mod io;
mod combined_data;
use io::{bam_io, pod5_io};

fn main() {
    let path: &str = "example_data/can_reads.pod5";
    let read_dataset = pod5_io::ReadDataset::from_pod5(path).unwrap();

    let path = "example_data/can_mappings.bam";
    let mut bam_index = bam_io::BamIndex::new(path).unwrap();

    let read_id = "6e37823a-9398-4be8-b111-65cab029f4e0";
    let pod5_read = read_dataset.get(read_id).unwrap();
    let bam_read = bam_index.get(read_id).unwrap();
    let mut combined_read = combined_data::CombinedRead::from_pod5_and_bam_record(pod5_read, &bam_read, false).unwrap();
    // println!("{:?}", combined_read.get_query_to_signal().unwrap().len());        
    
    let _ = combined_read.align_to_query();
    let _ = combined_read.align_to_reference();

    for read_id in read_dataset.keys() {
        let pod5_read = read_dataset.get(read_id).unwrap();
        let bam_read = bam_index.get(read_id).unwrap();
        let mut combined_read = combined_data::CombinedRead::from_pod5_and_bam_record(pod5_read, &bam_read, false).unwrap();
        let _ = combined_read.align_to_query().unwrap();
        let _ = combined_read.align_to_reference().unwrap();

        println!("{}\n{:?}\n{:?}\n\n", 
            read_id, 
            combined_read.get_query_to_signal().unwrap(), 
            combined_read.get_ref_to_signal().unwrap())
    }
}