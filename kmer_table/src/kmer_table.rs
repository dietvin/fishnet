/*!
 * This module provides functionality for loading, storing, and querying k-mers with their
 * associated level values. 
 * 
 * # Key Features
 * 
 * - **File-based loading**: Reads k-mers and levels from tab-delimited files
 * - **Binary representation**: Uses efficient binary encoding for k-mer storage and comparison
 * - **Sorted indexing**: Maintains k-mers sorted by their level values for efficient access
 * - **Level normalization**: Supports median absolute deviation (MAD) normalization
 * - **Dominant base analysis**: Identifies the most influential position within k-mers
 * - **Sequence-level extraction**: Extracts expected levels for entire DNA sequences
 * 
 * # Data Structure
 * 
 * The `KmerTable` struct stores:
 * - A hash map index for O(1) k-mer lookups
 * - Vectors of k-mers and their levels, sorted by level values
 * - Metadata including k-mer length and dominant base position
 * 
 * # File Format
 * 
 * Expected input files should be tab-delimited with two columns:
 * - Column 1: K-mer sequence (DNA string)
 * - Column 2: Level value (floating-point number)
 * 
 * # Usage Example
 * 
 * ```ignore
 * use std::path::PathBuf;
 * 
 * let path = PathBuf::from("levels.txt");
 * let mut table = KmerTable::new(&path)?;
 * table.fix_gauge()?;  // Normalize levels
 * 
 * let level = table.get("ATCGN")?;  // Look up k-mer level
 * let levels = table.extract_levels(b"ATCGATCG")?;  // Extract levels for sequence
 * ```
 */

use std::{collections::HashMap, path::PathBuf};
use helper::logger::get_log_vector_sample;
use crate::{binary_kmer::BinaryKmer, error::KmerTableError, kmer_table_data::KmerTableData};

#[derive(Debug)]
pub enum KmerSource {
    Embedded(&'static str),
    File(PathBuf)
}

/// A data structure for storing and querying k-mers with their associated levels
///
/// This structure reads k-mers and their associated levels from a tab-delimited file,
/// sorts them by level, and provides methods to query the level for a given k-mer.
#[derive(Debug)]
pub struct KmerTable{
    /// Mapping from k-mer strings to their indices in the vectors
    index: HashMap<BinaryKmer, usize>,
    /// Vector of k-mer strings sorted by level
    kmers: Vec<BinaryKmer>,
    /// Vector of level values corresponding to the k-mers
    levels: Vec<f32>,
    /// The length of k-mers stored in this table
    k: usize,
    /// Index of the position in k-mers that has the most influence on levels
    dominant_base: usize,
    /// Path to the underlying file
    source: KmerSource
}

impl KmerTable {
    pub(crate) fn new(
        index: HashMap<BinaryKmer, usize>,
        kmers: Vec<BinaryKmer>,
        levels: Vec<f32>,
        k: usize,
        dominant_base: usize,
        source: KmerSource,
    ) -> Self {
        KmerTable { index, kmers, levels, k, dominant_base, source }
    }

    /// Creates a new KmerTable from a file path
    ///
    /// Reads k-mers and their levels from a tab-delimited file, validates them,
    /// and constructs a sorted and indexed table for efficient lookups.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the tab-delimited file containing k-mers and levels
    ///
    /// # Returns
    ///
    /// * `Result<Self, KmerTableError>` - A new KmerTable instance or an error
    ///
    /// # Errors
    ///
    /// * `KmerTableError::FileNotFound` - If the file cannot be opened
    /// * `KmerTableError::EmptyFile` - If the file exists but contains no data
    /// * `KmerTableError::EmptyKmer` - If a k-mer in the file is an empty string
    /// * `KmerTableError::EvenKmer` - If a k-mer has even length (odd length is expected)
    /// * `KmerTableError::NonUniformKmerLength` - If k-mers have inconsistent lengths
    /// * `KmerTableError::DuplicateKmer` - If a k-mer appears multiple times in the file
    /// * `KmerTableError::MissingEntries` - If the number of k-mers is less than expected (4^k)
    /// * `KmerTableError::LineParsingError` - If a line doesn't have exactly 2 columns
    /// * `KmerTableError::FloatConversionError` - If a level value cannot be parsed as a float
    /// * `KmerTableError::BinaryKmerError` - If there's an error in the binary representation of a k-mer
    pub fn from_file(path: &PathBuf, do_fix_gauge: bool) -> Result<Self, KmerTableError> {
        log::info!("Initializing KmerTable from path: {}", path.display());

        let kmer_table_data = KmerTableData::from_file(
            path, 
            false, 
            do_fix_gauge
        )?;

        let kmer_table = kmer_table_data.into_kmer_table(KmerSource::File(path.clone()));
        
        log::debug!(
            "Initialized KmerTable: k = {}, kmers = {}, levels = {}, dominant_base = {}",
            kmer_table.k,
            get_log_vector_sample(&kmer_table.kmers.iter().map(|el| el.to_string()).collect::<Vec<String>>(), 10),
            get_log_vector_sample(&kmer_table.levels, 10),
            kmer_table.dominant_base
        );

        Ok(kmer_table)
    }

    /// Retrieves the level for a given BinaryKmer object
    ///
    /// # Arguments
    ///
    /// * `kmer` - The BinaryKmer to look up
    ///
    /// # Returns
    ///
    /// * `Result<&f32, KmerTableError>` - The level value associated with the k-mer, or an error
    ///
    /// # Errors
    ///
    /// * `KmerTableError::InvalidKmerLen` - If the provided k-mer has an incorrect length
    /// * `KmerTableError::IndexError` - If the provided k-mer is not found in the table
    pub fn get_from_binarykmer(&self, kmer: &BinaryKmer) -> Result<&f32, KmerTableError> {
        if kmer.k() != self.k {
            Err(KmerTableError::InvalidKmerLen(kmer.k(), self.k))
        } else {
            let idx = self.index.get(kmer).ok_or(
                KmerTableError::IndexError(kmer.to_string())
            )?;
            let level = &self.levels[*idx];
            Ok(level)
        }
    }

    /// Retrieves the level for a given k-mer
    ///
    /// Transform the string into a BinaryKmer and calls `get_from_binarykmer` 
    /// 
    /// # Arguments
    ///
    /// * `kmer` - The k-mer string to look up
    ///
    /// # Returns
    ///
    /// * `Result<&f32, KmerTableError>` - The level value associated with the k-mer, or an error
    ///
    /// # Errors
    ///
    /// * `KmerTableError::InvalidKmerLen` - If the provided k-mer has an incorrect length
    /// * `KmerTableError::IndexError` - If the provided k-mer is not found in the table
    /// * `KmerTableError::BinaryKmerError` - If there's an error creating the binary representation of the k-mer    
    pub fn get(&self, kmer: &str) -> Result<&f32, KmerTableError> {
        let binary_kmer = BinaryKmer::from_string(kmer)?;
        self.get_from_binarykmer(&binary_kmer)
    }

    /// Returns the string representation of the stored k-mers
    ///
    /// The k-mers are sorted by their level values.
    ///
    /// # Returns
    ///
    /// * `Vec<String>` - Reference to the vector of k-mers sorted by level
    pub fn kmers(&self) -> Vec<String> {
        self.kmers.iter().map(|bin| bin.to_string()).collect::<Vec<String>>()
    }

    /// Returns a reference to the vector of levels
    ///
    /// The levels are sorted in ascending order.
    ///
    /// # Returns
    ///
    /// * `&Vec<f32>` - Reference to the vector of levels in sorted order
    pub fn levels(&self) -> &Vec<f32> {
        &self.levels
    }

    /// Returns the length of k-mers in this table
    ///
    /// # Returns
    ///
    /// * `usize` - The length of k-mers (k)
    pub fn k(&self) -> usize {
        self.k
    }

    /// Returns the index of the position in k-mers that has the most influence on levels
    ///
    /// # Returns
    ///
    /// * `usize` - The index of the dominant base position
    pub fn dominant_base(&self) -> usize {
        self.dominant_base
    }


    /// Returns the source as a str. 
    ///
    /// # Returns
    ///
    /// * `&str` - Path to the kmer table file (if loaded from file) or the table name
    ///            (if using a precompiled table)
    pub fn source_str(&self) -> &str {
        match &self.source {
            KmerSource::File(path) => {
                let path_str = match path.to_str() {
                    Some(path_str) => path_str,
                    None => "Could not determine file path" 
                };
                return path_str;
            },
            KmerSource::Embedded(name) => {
                return *name;
            }
        }
    }


    /// Extracts the expected levels for a given sequence
    ///
    /// This function computes the level values for each position in the input sequence 
    /// by sliding a k-mer window across the sequence and looking up the corresponding levels
    /// in the table. The levels are aligned such that the dominant base of each k-mer
    /// corresponds to its position in the output vector.
    ///
    /// # Arguments
    ///
    /// * `seq` - A byte slice representing the DNA sequence
    ///
    /// # Returns
    ///
    /// * `Result<Vec<f32>, KmerTableError>` - A vector of level values for each position 
    ///   in the sequence, or an error
    ///
    /// # Errors
    ///
    /// * `KmerTableError::BinaryKmerError` - If there's an error creating the binary 
    ///   representation of a k-mer from the sequence
    /// * `KmerTableError::IndexError` - If a k-mer from the sequence is not found in the table
    ///
    /// # Notes
    ///
    /// * Positions before the dominant base position will have a level value of 0
    /// * The function assumes the input sequence is at least as long as k    
    pub fn extract_levels(&self, seq: &[u8]) -> Result<Vec<f32>, KmerTableError> {
        if seq.len() < self.k {
            return Err(KmerTableError::InvalidSeqLen(seq.len(), self.k));
        }
        let mut level_vec = vec![0f32; seq.len()];

        // Initital setup with k=5 & dominant_base=2:
        //  A A A A A A 
        //  0 1 2 3 4 5
        //  |   |     | 
        //  s   m     e(not incl)
        for pos in 0..(seq.len() - self.k + 1) {
            let center_pos = pos + self.dominant_base;
            let kmer = BinaryKmer::from_ascii(&seq[pos..(pos + self.k)])?;
            let level = self.get_from_binarykmer(&kmer)?;
            level_vec[center_pos] = *level;
        }

        Ok(level_vec)        
    }
}
