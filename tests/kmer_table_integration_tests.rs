use std::path::Path;
use fishnet::error::refinement_errors::kmer_table_errors::KmerTableError;
use fishnet::refinement::kmer_table::KmerTable;

#[test]
fn test_valid_kmer_table() {
    let path = Path::new("tests/kmer_tables/valid.txt");
    let result = KmerTable::new(path.to_str().unwrap());
    
    assert!(result.is_ok(), "Failed to create KmerTable from valid file");
    
    let table = result.unwrap();
    
    // Test that we can retrieve a level for a valid k-mer
    // Note: This assumes that the valid.txt file contains at least one k-mer
    let kmer = table.kmers()[0].clone();
    let level_result = table.get(&kmer);
    
    assert!(level_result.is_ok(), "Failed to get level for valid k-mer");
    assert_eq!(level_result.unwrap(), &table.levels()[0]);
    
    // Test that kmers and levels vectors have the same length
    assert_eq!(table.kmers().len(), table.levels().len(), "Kmers and levels vectors should have the same length");
}

#[test]
fn test_missing_entries_kmer_table() {
    let path = Path::new("tests/kmer_tables/invalid1_missing_entries.txt");
    let result = KmerTable::new(path.to_str().unwrap());
    
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
    let path = Path::new("tests/kmer_tables/invalid2_invalid_kmer.txt");
    let result = KmerTable::new(path.to_str().unwrap());
    
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
    let path = Path::new("tests/kmer_tables/invalid3_empty.txt");
    let result = KmerTable::new(path.to_str().unwrap());
    
    assert!(result.is_err(), "KmerTable creation should fail with empty file");
}

#[test]
fn test_nonexistent_kmer_table() {
    let path = Path::new("tests/kmer_tables/nonexistent.txt");
    let result = KmerTable::new(path.to_str().unwrap());
    
    assert!(result.is_err(), "KmerTable creation should fail with nonexistent file");
    
    match result {
        Err(KmerTableError::FileNotFound(_)) => {},
        _ => panic!("Expected FileNotFound error")
    }
}

#[test]
fn test_get_invalid_kmer() {
    let path = Path::new("tests/kmer_tables/valid.txt");
    let result = KmerTable::new(path.to_str().unwrap());
    
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
    match nonexistent_result {
        Err(KmerTableError::IndexError(_)) => {},
        _ => panic!("Expected IndexError")
    }
}

#[test]
fn test_kmer_table_sorted_by_level() {
    let path = Path::new("tests/kmer_tables/valid.txt");
    let result = KmerTable::new(path.to_str().unwrap());
    
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