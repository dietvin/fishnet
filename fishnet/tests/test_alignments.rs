use fishnet::core::loader::bam::BamFileLazy;
use fishnet::core::alignment::aligned_read::AlignedRead;
use pod5_reader_api::dataset::Pod5Dataset;

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn parse_directory(dirname: &PathBuf) -> Vec<PathBuf> {
    let mut pod5_files = vec![];
    if let Ok(entries) = fs::read_dir(dirname) {
        for entry in entries.flatten() {
            let path = entry.path();

            let abs_path = match path.canonicalize() {
                Ok(canonical) => canonical,
                Err(e) => {
                    log::warn!("Failed to canonicalize path '{}': {}", path.display(), e);
                    continue;
                }
            };

            if abs_path.is_dir() {
                let mut subdir_files = parse_directory(&abs_path);
                pod5_files.append(&mut subdir_files);
            } else if valid_file(&abs_path) {
                pod5_files.push(abs_path);
            }
        }
    } else {
        log::warn!("Failed to read directory: {:?}", dirname);
    }
    pod5_files
}

fn valid_file(path: &PathBuf) -> bool {
    // Check if the path if a filepath
    if !path.is_file() {
        log::warn!("Invalid file path: '{:?}' (Path is not a file path)", path.to_str());
        false
    } 
    // Check if the file exists
    else if !path.exists() {
        log::warn!("Invalid file path: '{:?}' (File does not exist)", path.to_str());
        false
    } 
    // Check if the file is a pod5 file
    else if !valid_extention(path) {
        log::warn!("Invalid file path: '{:?}' (Expected 'pod5' extension)", path.to_str());
        false
    } else {
        true
    }
}

fn valid_extention(path: &PathBuf) -> bool {
    if let Some(ext) = path.extension() {
        if ext == "pod5" {
            return true;
        }
    }
    false
}



#[test]
fn test_query_to_signal() {
    let paths = parse_directory(&PathBuf::from("../example_data"));
    let mut pod5_dataset = Pod5Dataset::new(&paths).unwrap();
    let mut bam_file = BamFileLazy::new(&PathBuf::from("../example_data/can_mappings.bam")).unwrap();

    for pod5_file in pod5_dataset.iter_files_mut() {
        let read_iterator = pod5_file.iter_reads().unwrap();
        for read_res in read_iterator {
            match read_res {
                Ok(mut pod5_read) => {
                    let read_id = pod5_read.read_id_string();
                    let mut bam_read = bam_file.get(&read_id).unwrap();

                    let mut aligned_read = AlignedRead::new(
                        &mut pod5_read, 
                        &mut bam_read, 
                        false
                    ).unwrap();

                    aligned_read.align_query_to_signal().unwrap();
                    let query_to_signal = aligned_read.query_to_signal().unwrap();

                    let expected_mapping_path = format!(
                        "tests/alignments/{}_query_to_signal.txt",
                        read_id
                    );
                    let expected_mapping = vec_from_file(&expected_mapping_path);
                    assert_eq!(*query_to_signal, expected_mapping);
                },
                Err(err) => eprintln!("Failed to extract read: {err}")
            }
        }
    }
}


#[test]
fn test_ref_to_signal() {
    let paths = parse_directory(&PathBuf::from("../example_data"));
    let mut pod5_dataset = Pod5Dataset::new(&paths).unwrap();
    let mut bam_file = BamFileLazy::new(&PathBuf::from("../example_data/can_mappings.bam")).unwrap();

    for pod5_file in pod5_dataset.iter_files_mut() {
        let read_iterator = pod5_file.iter_reads().unwrap();
        for read_res in read_iterator {
            match read_res {
                Ok(mut pod5_read) => {
                    let read_id = pod5_read.read_id_string();
                let mut bam_read = bam_file.get(&read_id).unwrap();

                let mut aligned_read = AlignedRead::new(
                    &mut pod5_read, 
                    &mut bam_read, 
                    false
                ).unwrap();

                aligned_read.align_query_to_signal().unwrap();
                aligned_read.align_reference_to_signal().unwrap();
                let ref_to_signal = aligned_read.reference_to_signal().unwrap();

                let expected_mapping_path = format!(
                    "tests/alignments/{}_ref_to_signal.txt",
                    read_id
                );
                let expected_mapping = vec_from_file(&expected_mapping_path);
                assert_eq!(*ref_to_signal, expected_mapping);
                },
                Err(err) => eprintln!("Failed to extract read: {err}")
            }
        }
    }
}


fn vec_from_file(path: &str) -> Vec<usize> {
    let file = File::open(path).expect("File not found");
    let reader = BufReader::new(file);

    let vec = reader
        .lines()
        .map(|line| line.unwrap().parse::<usize>().unwrap())
        .collect::<Vec<usize>>();
    vec
}