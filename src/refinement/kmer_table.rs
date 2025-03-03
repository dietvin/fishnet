use std::{collections::{HashMap, HashSet}, fs::File, io::{BufRead, BufReader}};
use super::super::error::refinement_errors::kmer_table_errors::KmerTableError;

/// A data structure for storing and querying k-mers with their associated levels
///
/// This structure reads k-mers and their associated levels from a tab-delimited file,
/// sorts them by level, and provides methods to query the level for a given k-mer.
pub struct KmerTable{
    /// Mapping from k-mer strings to their indices in the vectors
    index: HashMap<String, usize>,
    /// Vector of k-mer strings sorted by level
    kmers: Vec<String>,
    /// Vector of level values corresponding to the k-mers
    levels: Vec<f32>,
    /// The length of k-mers stored in this table
    k: usize
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
    /// * `KmerTableError::NonUniformKmerLength` - If k-mers have inconsistent lengths
    /// * `KmerTableError::DuplicateKmer` - If the k-mer has already been processed
    /// * `KmerTableError::MissingEntries` - If the number of k-mers is less than expected (4^k)
    /// * Various other errors from processing lines in the file
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
                        if v != kmer.len() {
                            return Err(
                                KmerTableError::NonUniformKmerLength(kmer.len(), v)
                            );
                        }    
                    },
                    None => prev_kmer_len = Some(kmer.len())
                }
    
                if !unique_kmers.insert(kmer.clone()) {
                    return Err(KmerTableError::DuplicateKmer(kmer.to_string()));
                }

                kmers_unsorted.push(kmer);
                levels_unsorted.push(level);    
            }
        }

        let k = kmers_unsorted[0].len();
        let exp_len = (4u32.pow(k as u32)) as usize;
        if kmers_unsorted.len() < exp_len {
            return Err(KmerTableError::MissingEntries(kmers_unsorted.len(), exp_len));
        }

        let (index, kmers_sorted, levels_sorted) = sort_and_index(
            &kmers_unsorted, 
            &levels_unsorted
        );

        Ok(KmerTable {
            index,
            kmers: kmers_sorted,
            levels: levels_sorted,
            k
        })
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
    pub fn get(&self, kmer: &str) -> Result<&f32, KmerTableError> {
        if kmer.len() != self.k {
            Err(KmerTableError::InvalidKmerLen(kmer.len(), self.k))
        } else {
            let idx = self.index.get(kmer).ok_or(
                KmerTableError::IndexError(kmer.to_string())
            )?;
            let level = &self.levels[*idx];
            Ok(level)
        }
    }

    /// Returns a reference to the vector of k-mers
    ///
    /// The k-mers are sorted by their level values.
    ///
    /// # Returns
    ///
    /// * `&Vec<String>` - Reference to the vector of k-mers sorted by level
    pub fn kmers(&self) -> &Vec<String> {
        &self.kmers
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
}


/// Processes one line from the kmer table 
/// 
/// When coming from the BufReader::lines function the trailing linebreaks are
/// already removed and each line should have the form: 
/// 
/// `KMER\tLEVEL`
/// 
/// # Arguments
/// * `line` - String containing the line to process
/// 
/// # Returns
/// * `Result<(String, f32), KmerTableError>` - Tuple containing the kmer and level value
/// 
/// # Errors
///
/// * `KmerTableError::LineParsingError` - If the number of columns is other than 2
/// * `KmerTableError::EmptyKmer` - If the kmer is empty ("")
/// * `KmerTableError::EvenKmer` - If k is even (odd k-mers are expected)
/// * `KmerTableError::FloatConversionError` - If the level can not be converted to a float
fn process_line(line: String) -> Result<(String, f32), KmerTableError> {
    let line_parts = line.split("\t").collect::<Vec<&str>>();
    
    // Check the number of columns (should be 2)
    if line_parts.len() != 2 {
        return Err(KmerTableError::LineParsingError(line_parts.len()));
    }

    let kmer = line_parts[0].to_string();
    let kmer_len = kmer.len();
    if kmer_len == 0 {
        return Err(KmerTableError::EmptyKmer);
    } else if (kmer_len % 2) == 0 {
        return Err(KmerTableError::EvenKmer(kmer_len));
    } 

    let level = line_parts[1].parse::<f32>()?;

    Ok((kmer, level))
}

/// Sorts k-mers by their levels and creates an index map for efficient lookups
///
/// Creates a new ordering of k-mers sorted by their levels and builds a mapping
/// from k-mer strings to their new indices in the sorted arrays.
///
/// # Arguments
///
/// * `kmers` - Vector of k-mer strings
/// * `levels` - Vector of level values corresponding to the k-mers
///
/// # Returns
///
/// * `(HashMap<String, usize>, Vec<String>, Vec<f32>)` - Tuple containing:
///   * A HashMap mapping k-mer strings to their indices in the sorted arrays
///   * A vector of k-mer strings sorted by level
///   * A vector of level values in sorted order
fn sort_and_index(kmers: &Vec<String>, levels: &Vec<f32>) -> (HashMap<String, usize>, Vec<String>, Vec<f32>) {
    let mut indices = (0..levels.len()).collect::<Vec<usize>>();
    indices.sort_by(
        |&i, &j| levels[i]
            .partial_cmp(&levels[j])
            .unwrap_or(std::cmp::Ordering::Equal)
    );
    
    let mut index = HashMap::new();

    let mut kmers_sorted = Vec::with_capacity(kmers.len());
    let mut levels_sorted = Vec::with_capacity(levels.len());

    for(i, &idx) in indices.iter().enumerate() {
        let kmer = &kmers[idx];
        let level = levels[idx];

        kmers_sorted.push(kmer.clone());
        levels_sorted.push(level);

        index.insert(kmer.clone(), i);
    }

    (index, kmers_sorted, levels_sorted)
}