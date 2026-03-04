/*!
 * This module contains functions for parsing an processing a kmer table.
 */

use serde::{Deserialize, Serialize};

use crate::{binary_kmer::BinaryKmer, error::{FixGaugeError, KmerTableDataError}, kmer_table::{KmerSource, KmerTable}};
use std::{collections::{HashMap, HashSet}, fs::File, io::{BufRead, BufReader}, path::PathBuf};

/// Serializable KmerTable
#[derive(Serialize, Deserialize)]
pub struct KmerTableData {
    /// Kmers sorted by their corresponding levels
    kmers: Vec<BinaryKmer>,
    /// Sorted levels of the kmers 
    levels: Vec<f32>,
    /// Number of bases in a kmer
    k: usize,
    /// The index of the dominant base in the kmer table
    dominant_base: usize
}

impl KmerTableData {
    /// Extracts data needed to initialize a `KmerTable` from a kmer table file.
    /// 
    /// Parses a kmer levels table, extracting the expected levels for each kmer,
    /// sorting levels and associated kmers by the levels. If specified levels are
    /// normalized. The kmer position that has the most influence on the levels is
    /// determined. Parsed data is returned in a tuple for initializing a `KmerTable`
    /// object or writing the info to binary files during build.
    /// 
    /// # Arguments
    /// * `path` - A PathBuf containing the path to a kmer table
    /// * `skip_header` - Whether to skip a header line (needed in legacy kmer tables)
    /// * `do_fix_gauge` - Whether to normalize the levels (needed in legacy kmer tables)
    /// 
    /// # Returns
    /// * `Result<(Vec<BinaryKmer>, Vec<f32>, usize, usize)>` - A tuple containing:
    ///     * A vector of kmers encoded as BinaryKmers sorted by their levels in the levels vector
    ///     * A vector containing the levels sorted in ascending order
    ///     * The k value
    ///     * The dominant base in the kmer table
    /// 
    /// # Errors
    /// * `KmerTableError::NonUniformKmerLength` - If the kmer lengths vary between lines
    /// * `KmerTableError::DuplicateKmer` - If a kmer occurs more than once
    /// * `KmerTableError::EmptyFile` - If the file is empty
    /// * `KmerTableError::MissingEntries` - If there are kmers missing in the file
    pub fn from_file(
        path: &PathBuf,
        skip_header: bool,
        do_fix_gauge: bool    
    ) -> Result<Self, KmerTableDataError> {
        let file = File::open(path)?;
        let file_buffer = BufReader::new(file);
    
        let mut unique_kmers = HashSet::new();
    
        let mut prev_kmer_len = None;
    
        let mut kmers_unsorted = Vec::new();
        let mut levels_unsorted = Vec::new();
    
        // Use the skip header as a proxy to indicate legacy format
        let is_legacy = skip_header;
        let mut skip_header_mut = skip_header; 
        for line in file_buffer.lines() {
            if skip_header_mut {
                skip_header_mut = false;
                continue;
            }
            
            let line = line?;
            if line.len() > 0 {
                let (kmer, level) = process_line(
                    line,
                    is_legacy
                )?;
    
                match prev_kmer_len {
                    Some(v) => {
                        if v != kmer.k() {
                            return Err(
                                KmerTableDataError::NonUniformKmerLength(kmer.k(), v)
                            );
                        }    
                    },
                    None => prev_kmer_len = Some(kmer.k())
                }
    
                if !unique_kmers.insert(kmer.clone()) {
                    return Err(KmerTableDataError::DuplicateKmer(kmer.to_string()));
                }
    
                kmers_unsorted.push(kmer);
                levels_unsorted.push(level);    
            }
        }
    
        if kmers_unsorted.len() == 0 {
            return Err(KmerTableDataError::EmptyFile);
        }
        let k = kmers_unsorted[0].k();
    
        let exp_len = (4u32.pow(k as u32)) as usize;
        if kmers_unsorted.len() < exp_len {
            return Err(KmerTableDataError::MissingEntries(kmers_unsorted.len(), exp_len));
        }
    
        let (kmers_sorted, mut levels_sorted) = sort_by_levels(
            &kmers_unsorted, 
            &levels_unsorted
        );
    
        if do_fix_gauge {
            fix_gauge(&mut levels_sorted)?
        }
    
        let dominant_base = determine_dominant_base(&kmers_sorted, k)?;
    
        Ok(KmerTableData {
            kmers: kmers_sorted, 
            levels: levels_sorted, 
            k, 
            dominant_base 
        })    
    }

    /// Transforms self into a KmerTable.
    /// 
    /// Calculates the index from the provided sorted kmers and uses it, the provided
    /// KmerSource and the data already present in self to initialize a KmerTable.
    /// 
    /// # Arguments
    /// * `kmer_source` - A KmerSource indicating a table parsed from a file or an embedded
    ///     table
    /// 
    /// # Returns
    /// * `KmerTable` - The newly initialized kmer table
    pub fn into_kmer_table(self, kmer_source: KmerSource) -> KmerTable {
        let index = index_sorted_kmers(&self.kmers);
        KmerTable::new(
            index,
            self.kmers,
            self.levels,
            self.k,
            self.dominant_base,
            kmer_source
        )
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
fn process_line(line: String, is_legacy: bool) -> Result<(BinaryKmer, f32), KmerTableDataError> {
    let line_parts = line.split("\t").collect::<Vec<&str>>();
    
    // Check the number of columns (should be 2, or 6/7 for legacy models)
    if (
        !is_legacy && line_parts.len() != 2
    ) || (
        // Some legacy tables don't have the 'weight' column
        is_legacy && (line_parts.len() != 6 && line_parts.len() != 7)
    ) {
        return Err(KmerTableDataError::LineParsingError(line_parts.len()));
    }

    let kmer = BinaryKmer::from_string(line_parts[0])?;
    let kmer_len = kmer.k();
    if kmer_len == 0 {
        return Err(KmerTableDataError::EmptyKmer);
    } 
    // Removed check for even kmer length, since this isn't done in Remora...
    // else if (kmer_len % 2) == 0 {
        // return Err(KmerTableDataError::EvenKmer(kmer_len));
    // } 

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
fn sort_by_levels(kmers: &Vec<BinaryKmer>, levels: &Vec<f32>) -> (
    Vec<BinaryKmer>, 
    Vec<f32>
) {
    let mut indices = (0..levels.len()).collect::<Vec<usize>>();
    indices.sort_by(
        |&i, &j| levels[i]
            .partial_cmp(&levels[j])
            .unwrap_or(std::cmp::Ordering::Equal)
    );
    
    let mut kmers_sorted = Vec::with_capacity(kmers.len());
    let mut levels_sorted = Vec::with_capacity(levels.len());

    for &idx in indices.iter() {
        let kmer = &kmers[idx];
        let level = levels[idx];

        kmers_sorted.push(kmer.clone());
        levels_sorted.push(level);
    }

    (kmers_sorted, levels_sorted)
}

/// Creates an mapping of kmers to their index
/// 
/// Iterates through the **sorted** kmers as set up
/// in `sort_by_levels`.
fn index_sorted_kmers(
    kmers_sorted: &Vec<BinaryKmer>
) -> HashMap<BinaryKmer, usize> {
    let mut index = HashMap::new();

    for (i, kmer) in kmers_sorted.iter().enumerate() {
        index.insert(kmer.clone(), i);
    }

    index
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
fn determine_dominant_base(kmers_sorted: &Vec<BinaryKmer>, k: usize) -> Result<usize, KmerTableDataError> {
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
        KmerTableDataError::ArgMaxError
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

pub trait Median {
    fn median(&self) -> Option<f32>;
}

impl Median for [f32] {
    fn median(&self) -> Option<f32> {
        let len = self.len();
        if len == 0 {
            return None;
        }
        
        let mut sorted = self.to_vec();
        sorted.sort_by(
            |a, b| a
                .partial_cmp(b)
                .unwrap_or(std::cmp::Ordering::Equal)
        );
        
        Some(if len % 2 == 1 {
            sorted[len / 2]
        } else {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
        })
    }
}

const SCALE_FACTOR: f32 = 1.4826;

/// Standardizes provdided levels in place.
/// 
/// # Arguments
/// * `levels` - A reference to a f32 vector containing signal levels
/// 
/// # Errors
/// * `FixGaugeError::MedianNone` - If the median cannot be calculated
/// * `FixGaugeError::MadNone` - If the mean absolute deviation cannot be calculated
/// * `FixGaugeError::ZeroDivision` - If the MAD is zero (which would result in a 0-division)
fn fix_gauge(
    levels: &mut [f32]
) -> Result<(), FixGaugeError> {
    let median = levels.median().ok_or(FixGaugeError::MedianNone)?;

    let mut mad = levels.iter().map(|el| (el - median).abs())
        .collect::<Vec<f32>>()
        .median()
        .ok_or(FixGaugeError::MadNone)?;

    mad *= SCALE_FACTOR;

    if mad == 0.0 {
        return Err(FixGaugeError::ZeroDivision);
    }

    for el in levels.iter_mut() {
        *el = (*el - median) / mad;
    }

    Ok(())
}







#[cfg(test)]
mod test {
    use super::{argmax, kruskal, Median};

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

    #[test]
    fn test_median1() {
        let vec = vec![1.0,2.0,3.0];
        let med = vec.median();

        assert_eq!(med, Some(2.0));
    }

    #[test]
    fn test_median2() {
        let vec = vec![1.0,2.0,3.0,4.0];
        let med = vec.median();

        assert_eq!(med, Some(2.5));
    }

    #[test]
    fn test_median3() {
        let vec: Vec<f32> = vec![];
        let med = vec.median();

        assert_eq!(med, None);
    }

    #[test]
    fn test_median4() {
        let vec: Vec<f32> = vec![0.0,1.0,1.0,2.0,3.0];
        let med = vec.median();

        assert_eq!(med, Some(1.0));
    }


}