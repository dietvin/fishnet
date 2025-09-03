use std::{collections::HashMap, fs::File, io::{BufRead, BufReader}, path::PathBuf};
use alignment::error::refinement_errors::kmer_table_errors::KmerTableError;
use alignment::core::refinement::kmer_table::KmerTable;
use approx::assert_relative_eq;
use serde::Deserialize;
use walkdir::WalkDir;


fn hashmap_from_file(path: &str) -> HashMap<String, f32> {
    let file = File::open(path).expect("File not found");
    let reader = BufReader::new(file);

    let map = reader
        .lines()
        .map(|line| process_line(line.unwrap()))
        .collect::<HashMap<String, f32>>();
    map
}

fn process_line(line: String) -> (String, f32) {
    let line = line.split("\t").collect::<Vec<&str>>();
    let kmer = line[0].to_string();
    let level = line[1].parse::<f32>().unwrap();
    (kmer, level)
}


#[test]
fn test_valid_kmer_table() {
    let path = PathBuf::from("tests/kmer_tables/valid.txt");
    let result = KmerTable::new(&path);
    assert!(result.is_ok(), "Failed to create KmerTable from valid file");
    
    let table = result.unwrap();
    
    // Test that we can retrieve a level for a valid k-mer
    let kmer = table.kmers()[0].clone();
    let level_result = table.get(&kmer);
    
    assert!(level_result.is_ok(), "Failed to get level for valid k-mer");
    assert_eq!(level_result.unwrap(), &table.levels()[0]);
    
    // Test that kmers and levels vectors have the same length
    assert_eq!(table.kmers().len(), table.levels().len(), "Kmers and levels vectors should have the same length");

    assert_eq!(table.dominant_base(), 6);
}


#[test]
fn test_missing_entries_kmer_table() {
    let path = PathBuf::from("tests/kmer_tables/invalid1_missing_entries.txt");
    let result = KmerTable::new(&path);
    
    assert!(result.is_err(), "KmerTable creation should fail with missing entries");
    
    match result {
        Err(KmerTableError::MissingEntries(actual, expected)) => {
            assert!(actual < expected, "Expected missing entries error with actual < expected");
        },
        _ => panic!("Expected MissingEntries error, got different error or success")
    }
}


#[test]
fn test_invalid_kmer_table() {
    let path = PathBuf::from("tests/kmer_tables/invalid2_invalid_kmer.txt");
    let result = KmerTable::new(&path);
    
    assert!(result.is_err(), "KmerTable creation should fail with invalid k-mer");
    
    // Test for possible error types (depending on what invalid2_invalid_kmer.txt contains)
    match result {
        Err(KmerTableError::NonUniformKmerLength(_, _)) => {},
        Err(KmerTableError::DuplicateKmer(_)) => {},
        Err(KmerTableError::EmptyKmer) => {},
        Err(KmerTableError::EvenKmer(_)) => {},
        Err(KmerTableError::LineParsingError(_)) => {},
        Err(KmerTableError::FloatConversionError(_)) => {},
        _ => panic!("Expected one of the kmer validation errors")
    }
}


#[test]
fn test_empty_kmer_table() {
    let path = PathBuf::from("tests/kmer_tables/invalid3_empty.txt");
    let result = KmerTable::new(&path);
    
    assert!(result.is_err(), "KmerTable creation should fail with empty file");
}


#[test]
fn test_nonexistent_kmer_table() {
    let path = PathBuf::from("tests/kmer_tables/nonexistent.txt");
    let result = KmerTable::new(&path);
    
    assert!(result.is_err(), "KmerTable creation should fail with nonexistent file");
    
    match result {
        Err(KmerTableError::FileNotFound(_)) => {},
        _ => panic!("Expected FileNotFound error")
    }
}


#[test]
fn test_get_invalid_kmer() {
    let path = PathBuf::from("tests/kmer_tables/valid.txt");
    let result = KmerTable::new(&path);
    
    assert!(result.is_ok(), "Failed to create KmerTable from valid file");
    
    let table = result.unwrap();
    
    // Test with an invalid length k-mer
    let invalid_length_result = table.get("A");
    assert!(invalid_length_result.is_err(), "Get should fail with invalid length k-mer");
    match invalid_length_result {
        Err(KmerTableError::InvalidKmerLen(actual, expected)) => {
            assert_eq!(actual, 1);
            assert_eq!(expected, table.kmers()[0].len());
        },
        _ => panic!("Expected InvalidKmerLen error")
    }
    
    // Test with a valid length but non-existent k-mer
    // Create a k-mer of the right length that's unlikely to be in the table
    let k = table.kmers()[0].len();
    let invalid_kmer = "N".repeat(k);
    let nonexistent_result = table.get(&invalid_kmer);
    
    assert!(nonexistent_result.is_err(), "Get should fail with non-existent k-mer");
}


#[test]
fn test_kmer_table_sorted_by_level() {
    let path = PathBuf::from("tests/kmer_tables/valid.txt");
    let result = KmerTable::new(&path);
    
    assert!(result.is_ok(), "Failed to create KmerTable from valid file");
    
    let table = result.unwrap();
    let levels = table.levels();
    
    // Check that levels are sorted in ascending order
    for i in 1..levels.len() {
        assert!(levels[i-1] <= levels[i], "Levels should be sorted in ascending order");
    }
    
    // Verify that if we look up each k-mer, we get the correct level
    for (i, kmer) in table.kmers().iter().enumerate() {
        let level_result = table.get(kmer);
        assert!(level_result.is_ok(), "Failed to get level for valid k-mer");
        assert_eq!(level_result.unwrap(), &levels[i]);
    }
}


#[test]
fn test_levels_as_expected() {
    let path = PathBuf::from("tests/kmer_tables/valid.txt");
    let result = KmerTable::new(&path);
    
    assert!(result.is_ok(), "Failed to create KmerTable from valid file");
    
    let table = result.unwrap();
    let levels = table.levels();
    
    let expected_levels = hashmap_from_file("tests/kmer_tables/levels.txt");

    assert_eq!(levels.len(), expected_levels.len());

    // Check if the levels are the same as given by the Remora implementation
    for kmer in table.kmers() {
        let level = table.get(&kmer).unwrap();
        let level_exp = expected_levels.get(&kmer).unwrap();
        assert!((level - level_exp).abs() < 10.0_f32.powi(-5))
    }
}


#[test]
fn test_levels_fix_gauge_as_expected() {
    let path = PathBuf::from("tests/kmer_tables/valid.txt");
    let result = KmerTable::new(&path);
    
    assert!(result.is_ok(), "Failed to create KmerTable from valid file");
    
    let mut table = result.unwrap();

    let result_fix_gauge = table.fix_gauge();
    assert!(result_fix_gauge.is_ok(), "Failed to normalize the levels");

    let levels = table.levels();
    
    let expected_levels = hashmap_from_file("tests/kmer_tables/levels_fix_gauge.txt");

    assert_eq!(levels.len(), expected_levels.len());

    // Check if the levels are the same as given by the Remora implementation
    for kmer in table.kmers() {
        let level = table.get(&kmer).unwrap();
        let level_exp = expected_levels.get(&kmer).unwrap();
        assert!((level - level_exp).abs() < 10.0_f32.powi(-5))
    }
}









#[derive(Debug, Deserialize)]
struct JsonData {
    int_seq: Vec<u8>,
    levels: Vec<f32>
}

fn load_json(path: &str) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();
    data
}

fn test_with_data_from(dirname: &str) {
    let dir = format!("tests/{}/kmer_table/extract_levels", dirname);

    let mut kmer_table = KmerTable::new(&PathBuf::from("../example_data/levels.txt")).unwrap();
    kmer_table.fix_gauge().unwrap();

    let mut files= WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<PathBuf>>();

    files.sort();

    for file in files {
        let path_str = file.to_str().unwrap();

        let data = load_json(&path_str);

        let seq = data.int_seq
            .iter()
            .map(|&el| {
                match el {
                    0 => 65,
                    1 => 67,
                    2 => 71,
                    3 => 84,
                    _ => 0
                }
            })
            .collect::<Vec<u8>>();

        let levels_result = kmer_table.extract_levels(&seq).unwrap();
        
        for i in 0..levels_result.len() - 1 {
            assert_relative_eq!(levels_result[i], data.levels[i], epsilon=0.1);
        }
    }
}

#[test]
fn test_kmer_extract_levels_querymap_theilsen() {
    test_with_data_from("test_data_querymap_theilsen");
}

#[test]
fn test_kmer_extract_levels_refmap_theilsen() {
    test_with_data_from("test_data_refmap_theilsen");
}

#[test]
fn test_kmer_extract_levels_querymap_leastsquares() {
    test_with_data_from("test_data_querymap_leastsquares");
}

#[test]
fn test_kmer_extract_levels_refmap_leastsquares() {
    test_with_data_from("test_data_refmap_leastsquares");
}