use std::{collections::HashMap, fs::File, io::{BufRead, BufReader}, os::unix::process};

use super::super::error::refinement_errors::kmer_table_errors::KmerTableError;

struct KmerTable{
    kmers: HashMap<String, usize>,
    levels: Vec<f32>,
    k: u16
}

impl KmerTable {
    pub fn new(path: &str) -> Result<Self, KmerTableError> {
        let file = File::open(path)?;
        let file_buffer = BufReader::new(file);

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
    
                kmers_unsorted.push(kmer);
                levels_unsorted.push(level);    
            }
        }

        // Sort the levels in ascending order, keeping track of the corresponding 
        // Kmers to fill an index hashmap. That way the values are sorted and it's 
        // still possible to get a value


        Ok(KmerTable {

        })
    }

    pub fn get(&self, &str) -> &f32 {}

    pub fn kmers(&self) -> &Vec<String> {}

    pub fn levels(&self) -> &Vec<f32> {}
}

/// Processes one line from the kmer table 
/// 
/// When coming from the BufReader::lines function the trailing linebreaks are
/// already removed and each line should have the form: 
/// 
/// `KMER\tLEVEL`
/// 
/// # Arguments
/// * `line` - String 
/// 
/// # Returns
/// * Result<(String, f32), KmerTableError> - Tuple containing the kmer and level value
/// 
/// # Errors
///
/// * `BamFileError::LineParsingError` - If the number of columns is other than 2
/// * `BamFileError::EmptyKmer` - If the kmer is empty ("")
/// * `BamFileError::EvenKmer` - If k is even
/// * `BamFileError::FloatConversionError` - If the level can not be converted to a float
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
    } else if (kmer_len % 2) != 0 {
        return Err(KmerTableError::EvenKmer(kmer_len));
    }

    let level = line_parts[1].parse::<f32>()?;

    Ok((kmer, level))
}