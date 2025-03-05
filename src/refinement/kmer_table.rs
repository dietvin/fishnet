mod helpers;
mod binary_kmer;

use std::{collections::{HashMap, HashSet}, fs::File, io::{BufRead, BufReader}};
use self::binary_kmer::BinaryKmer;
use self::helpers::{process_line, sort_and_index, determine_dominant_base, Median};
use super::super::error::refinement_errors::kmer_table_errors::KmerTableError;
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
    dominant_base: usize
}

impl KmerTable {
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
    pub fn new(path: &str) -> Result<Self, KmerTableError> {
        let file = File::open(path)?;
        let file_buffer = BufReader::new(file);

        let mut unique_kmers = HashSet::new();

        let mut prev_kmer_len = None;

        let mut kmers_unsorted = Vec::new();
        let mut levels_unsorted = Vec::new();

        // Read the kmer table line for line
        for line in file_buffer.lines() {
            let line = line?;
            if line.len() > 0 {
                let (kmer, level) = process_line(line)?;

                match prev_kmer_len {
                    Some(v) => {
                        if v != kmer.k() {
                            return Err(
                                KmerTableError::NonUniformKmerLength(kmer.k(), v)
                            );
                        }    
                    },
                    None => prev_kmer_len = Some(kmer.k())
                }
    
                if !unique_kmers.insert(kmer.clone()) {
                    return Err(KmerTableError::DuplicateKmer(kmer.to_string()));
                }

                kmers_unsorted.push(kmer);
                levels_unsorted.push(level);    
            }
        }

        if kmers_unsorted.len() == 0 {
            return Err(KmerTableError::EmptyFile);
        }
        let k = kmers_unsorted[0].k();

        let exp_len = (4u32.pow(k as u32)) as usize;
        if kmers_unsorted.len() < exp_len {
            return Err(KmerTableError::MissingEntries(kmers_unsorted.len(), exp_len));
        }

        let (index, kmers_sorted, levels_sorted) = sort_and_index(
            &kmers_unsorted, 
            &levels_unsorted
        );

        let dominant_base = determine_dominant_base(&kmers_sorted, k)?;

        Ok(KmerTable {
            index,
            kmers: kmers_sorted,
            levels: levels_sorted,
            k,
            dominant_base
        })
    }

    /// Normalizes the level values using median and median absolute deviation (MAD)
    ///
    /// This function adjusts the levels by subtracting the median and dividing by the MAD.
    /// It is useful for standardizing the data and making it more robust against outliers.
    ///
    /// # Returns
    ///
    /// * `Result<(), KmerTableError>` - Returns `Ok(())` if normalization succeeds, or an error if it fails.
    ///
    /// # Errors
    ///
    /// * `KmerTableError::FixGaugeError` - If the median cannot be determined.
    /// * `KmerTableError::FixGaugeError` - If the MAD cannot be determined.
    /// * `KmerTableError::FixGaugeError` - If the MAD is zero, which would result in division by zero.
    pub fn fix_gauge(&mut self) -> Result<(), KmerTableError> {
        let median = self.levels.median().ok_or(
            KmerTableError::FixGaugeError("Could not determine the median".to_string())
        )?;

        let mut mad = self.levels.iter().map(|el| (el - median).abs())
            .collect::<Vec<f32>>()
            .median()
            .ok_or(
                KmerTableError::FixGaugeError("Could not determine the MAD".to_string())
            )?;

        mad *= 1.4826; // Factor scales MAD to SD

        if mad == 0.0 {
            return Err(KmerTableError::FixGaugeError("Zero division".to_string()));
        }

        self.levels = self.levels.iter()
            .map(|el| (el - median) / mad)
            .collect::<Vec<f32>>();
        
        Ok(())
    }

    /// Retrieves the level for a given k-mer
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
        if kmer.len() != self.k {
            Err(KmerTableError::InvalidKmerLen(kmer.len(), self.k))
        } else {
            let kmer_binary = BinaryKmer::from_string(kmer)?;
            let idx = self.index.get(&kmer_binary).ok_or(
                KmerTableError::IndexError(kmer.to_string())
            )?;
            let level = &self.levels[*idx];
            Ok(level)
        }
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

    pub fn extract_levels(&self, seq: &[u8]) -> Result<Vec<f32>, KmerTableError> {
        todo!()
    }
}
