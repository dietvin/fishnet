use fishnet::loader::{bam::BamFileLazy, pod5::Pod5Index};
use fishnet::alignment::aligned_read::AlignedRead;

use std::fs::File;
use std::io::{BufRead, BufReader};

#[test]
fn test_query_to_signal() {
    let pod5_index = Pod5Index::from_dir("example_data", false).unwrap();
    let mut bam_file = BamFileLazy::new("example_data/can_mappings.bam").unwrap();

    for read in pod5_index.reads() {
        match read {
            Ok((_, read_id, pod5_read)) => {
                let mut pod5_read = pod5_read;
                let mut bam_read = bam_file.get(&read_id).unwrap();

                let mut aligned_read = AlignedRead::new(
                    &mut pod5_read, 
                    &mut bam_read, 
                    false
                ).unwrap();

                aligned_read.align_query_to_signal().unwrap();
                let query_to_signal = aligned_read.query_to_signal().unwrap();

                let expected_mapping_path = format!(
                    "tests/expected_alignments/{}_query_to_signal.txt",
                    read_id
                );
                let expected_mapping = vec_from_file(&expected_mapping_path);
                assert_eq!(*query_to_signal, expected_mapping);
            },
            Err(err) => eprintln!("Failed to extract read: {err}")
        }
    }
    

}


fn vec_from_file(path: &str) -> Vec<usize> {
    let mut file = File::open(path).expect("File not found");
    let reader = BufReader::new(file);

    let vec = reader
        .lines()
        .map(|line| line.unwrap().parse::<usize>().unwrap())
        .collect::<Vec<usize>>();
    vec
}