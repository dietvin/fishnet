use std::{collections::{HashMap, HashSet}, fs::File, io::{BufRead, BufReader}};
use super::binary_kmer::BinaryKmer;
use super::super::error::refinement_errors::kmer_table_errors::KmerTableError;

/// A data structure for storing and querying k-mers with their associated levels
///
/// This structure reads k-mers and their associated levels from a tab-delimited file,
/// sorts them by level, and provides methods to query the level for a given k-mer.
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
/// * `Result<(BinaryKmer, f32), KmerTableError>` - Tuple containing the kmer and level value
/// 
/// # Errors
///
/// * `KmerTableError::LineParsingError` - If the number of columns is other than 2
/// * `KmerTableError::EmptyKmer` - If the kmer is empty ("")
/// * `KmerTableError::EvenKmer` - If k is even (odd k-mers are expected)
/// * `KmerTableError::FloatConversionError` - If the level can not be converted to a float
/// * `KmerTableError::BinaryKmerError` - If there's an error creating the binary representation of the k-mer
fn process_line(line: String) -> Result<(BinaryKmer, f32), KmerTableError> {
    let line_parts = line.split("\t").collect::<Vec<&str>>();
    
    // Check the number of columns (should be 2)
    if line_parts.len() != 2 {
        return Err(KmerTableError::LineParsingError(line_parts.len()));
    }

    let kmer = BinaryKmer::from_string(line_parts[0])?;
    let kmer_len = kmer.k();
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
fn sort_and_index(kmers: &Vec<BinaryKmer>, levels: &Vec<f32>) -> (HashMap<BinaryKmer, usize>, Vec<BinaryKmer>, Vec<f32>) {
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

/// Determines the position in k-mers that has the most influence on levels
///
/// This function analyzes each position in the k-mers and determines which position
/// has the strongest statistical association with the level values. It uses the
/// Kruskal-Wallis test to measure the association at each position.
///
/// # Arguments
///
/// * `kmers_sorted` - Vector of k-mer strings sorted by their level values
/// * `k` - The length of k-mers
///
/// # Returns
///
/// * `Result<usize, KmerTableError>` - The position (0-based index) with the strongest
///   association to level values, or an error
///
/// # Errors
///
/// * `KmerTableError::BinaryKmerError` - If there's an error accessing a nucleotide in the binary k-mer
/// * `KmerTableError::KruskalTestError` - If the Kruskal-Wallis test fails
/// * `KmerTableError::ArgMaxError` - If the maximum test statistic cannot be determined
fn determine_dominant_base(kmers_sorted: &Vec<BinaryKmer>, k: usize) -> Result<usize, KmerTableError> {
    let n_kmers = kmers_sorted.len();
    
    // Calculate test scores for each index in the kmer
    let mut kmer_stats = Vec::with_capacity(k);
    for base_idx in 0..k {
        // Collect the indices (sorted by the levels!) of the kmers where we have A/C/G/T an index
        // base_idx in the corresponding vectors
        let mut kmer_indices_a = Vec::with_capacity(n_kmers/4);
        let mut kmer_indices_c = Vec::with_capacity(n_kmers/4);
        let mut kmer_indices_g = Vec::with_capacity(n_kmers/4);
        let mut kmer_indices_t = Vec::with_capacity(n_kmers/4);

        for (kmer_idx, kmer) in kmers_sorted.iter().enumerate() {
            let char_at_base_idx = kmer.nucleotide_at(base_idx)?;

            match char_at_base_idx {
                'A' => kmer_indices_a.push(kmer_idx),
                'C' => kmer_indices_c.push(kmer_idx),
                'G' => kmer_indices_g.push(kmer_idx),
                'T' => kmer_indices_t.push(kmer_idx),
                _ => unreachable!(), // Only the four bases can occur in the implementation
            }
        }

        let test_statistic = kruskal(&[
            &kmer_indices_a, &kmer_indices_c, &kmer_indices_g, &kmer_indices_t
        ]);

        kmer_stats.push(test_statistic);
    }

    let dominant_base = argmax(&kmer_stats).ok_or(
        KmerTableError::ArgMaxError
    )?;

    Ok(dominant_base)
}

/// Performs the Kruskal-Wallis H test for comparing multiple groups. Calculates only the
/// test statistic H. Lower H values suggest more similarity between groups.
///
/// # Arguments
///
/// * `samples` - A slice of slices, where each inner slice represents a group of ranks.
///
/// # Returns
///
/// * `f64` - The calculated H statistic
///
/// # Formula
///
/// `H = [(12 / (N(N+1))) * Σ(Ri²/ni)] - 3(N+1)`
/// 
/// Where:
/// * `N` is the total number of ranks
/// * `Ri` is the sum of the ranks for group i
/// * `ni` is the number of ranks in group i
fn kruskal(samples: &[&[usize]]) -> f64 {
    let total_observations = samples.iter().map(|s| s.len() as f64).sum::<f64>();
        
    let sum = samples.iter().map(
        |group| 
            group.iter().map(|&el| el as f64).sum::<f64>().powi(2) / (group.len() as f64) 
    ).sum::<f64>();

    (12.0 / (total_observations * (total_observations + 1.0))) * sum - 3.0 * (total_observations + 1.0)
}

/// Finds the index of the maximum value in a slice of f32 values.
/// 
/// This function returns the index of the first occurrence of the maximum value.
/// If the slice is empty, it returns None.
/// If there are NaN values, they are handled by treating them as equal to other values
/// in the comparison (via the fallback in partial_cmp).
///
/// # Arguments
///
/// * `vec` - A slice of f32 values
///
/// # Returns
///
/// * `Option<usize>` - The index of the maximum value, or None if the slice is empty
///
/// # Examples
///
/// ```ignore
/// let numbers = vec![3.5, 1.0, 6.8, 2.3, 5.1];
/// let max_index = argmax(&numbers);
/// assert_eq!(max_index, Some(2)); // 6.8 is at index 2
///
/// let empty: Vec<f32> = vec![];
/// let max_index = argmax(&empty);
/// assert_eq!(max_index, None);
/// ```
fn argmax(vec: &[f64]) -> Option<usize> {
    vec.iter()
        .enumerate()
        .max_by(
            |(_, a), (_, b)| a
                .partial_cmp(b)
                .unwrap_or(std::cmp::Ordering::Equal)
        )
        .map(|(index, _)| index)
}










#[cfg(test)]
mod test {
    use super::{kruskal, argmax};

    /// First example from the scipy documentation
    #[test]
    fn test_kruskal1() {
        let x = vec![1, 3, 5, 7, 9];
        let y = vec![2, 4, 6, 8, 10];
    
        let h = kruskal(&[&x, &y]);
        assert!(h-0.2727272727272734<(10.0 as f64).powi(-5))
    }

    /// Second example from the scipy documentation
    #[test]
    fn test_kruskal2() {
        let x = vec![1, 1, 1];
        let y = vec![2, 2, 2];
        let z = vec![2, 2];
        let h = kruskal(&[&x, &y, &z]);
        assert!(h-7.0 < (10.0 as f64).powi(-5))
    }

    #[test]
    fn test_argmax1() {
        let numbers = vec![3.5, 1.0, 6.8, 2.3, 5.1];
        let max_index = argmax(&numbers);
        assert_eq!(max_index, Some(2)); // 6.8 is at index 2        
    }

    #[test]
    fn test_argmax2() {
        let empty: Vec<f64> = vec![];
        let max_index = argmax(&empty);
        assert_eq!(max_index, None);
    }

    #[test]
    fn test_argmax3() {
        let with_nan = vec![1.0, f64::NAN, 3.0, 2.0];
        let max_index = argmax(&with_nan);
        assert_eq!(max_index, Some(2));
    }

}