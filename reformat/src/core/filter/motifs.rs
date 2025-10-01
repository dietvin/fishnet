use std::{fs::File, io::{BufRead, BufReader}, path::PathBuf};

use crate::{core::filter::{motif::Motif, ChunkInfo}, error::core::filter::MotifsError, execute::config::FilterSource};

/// A collection of sequence motifs for sequence matching.
///
/// This struct is constructed from a `FilterSource` such as a FASTA file
/// or a list of motif strings. Motifs are validated to contain only valid
/// nucleotide characters (A, C, G, T) and can be searched within sequences.
#[derive(Debug)]
pub(crate) struct Motifs {
    motifs: Vec<Motif>
}

impl Motifs {
    /// Constructs a new `Motifs` instance from a given `FilterSource`.
    ///
    /// - `FilterSource::MotifFromFile` -> Reads motifs from a FASTA file.
    /// - `FilterSource::MotifFromInput` -> Creates motifs from a list of strings.
    ///
    /// # Arguments
    /// * `filter_source` - The source configuration specifying how to load motifs
    ///
    /// # Returns
    /// * `Result<Self, MotifsError>` - The constructed Motifs instance or an error
    ///
    /// # Errors
    /// Returns an error if the filter source is invalid or motif parsing fails.
    pub(crate) fn from_filter_source(filter_source: &FilterSource) -> Result<Self, MotifsError> {
        match filter_source {
            FilterSource::MotifFromFile { path } => Self::from_fasta(path),
            FilterSource::MotifFromInput { motifs } => Self::from_motifs(motifs),
            _ => return Err(MotifsError::InvalidFilterSource)
        }
    }

    /// Reads motifs from a FASTA file at the given path.
    ///
    /// Each sequence in the FASTA file becomes a motif. Header lines (starting with '>')
    /// provide motif names, and sequence lines contain the motif patterns.
    ///
    /// # Arguments
    /// * `path` - Path to the FASTA file containing motif sequences
    ///
    /// # Returns
    /// * `Result<Self, MotifsError>` - The constructed Motifs instance or an error
    ///
    /// # Errors
    /// Returns an error if the file cannot be read, FASTA format is invalid,
    /// or motif sequences contain invalid characters.
    fn from_fasta(path: &PathBuf) -> Result<Self, MotifsError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let mut motifs = Vec::new();
        let mut current_name: Option<String> = None;
        let mut current_seq = String::new();
    
        for line_res in reader.lines() {
            let line = line_res?;
            let line = line.trim();
    
            if line.starts_with('>') {
                // flush previous motif
                if let Some(name) = current_name.take() {
                    let motif = Motif::new(&name, &current_seq)?;
                    motifs.push(motif);
                }
    
                // start new motif
                current_name = Some(line[1..].to_string());
                current_seq.clear();
            } else if !line.is_empty() {
                current_seq.push_str(line);
            }
        }
    
        // flush last motif
        if let Some(name) = current_name {
            let motif = Motif::new(&name, &current_seq)?;
            motifs.push(motif);
        }
    
        Ok(Self { motifs })
    }

    /// Creates motifs from a list of motif strings.
    ///
    /// Each string becomes a motif with an auto-generated name (motif0, motif1, etc.).
    /// All motif strings are validated for valid nucleotide characters.
    ///
    /// # Arguments
    /// * `motifs` - Vector of motif strings to create motifs from
    ///
    /// # Returns
    /// * `Result<Self, MotifsError>` - The constructed Motifs instance or an error
    ///
    /// # Errors
    /// Returns an error if any motif string contains invalid characters.
    fn from_motifs(motifs: &Vec<String>) -> Result<Self, MotifsError> {
        let motifs = motifs.iter()
            .enumerate()
            .map(|(idx, m)| {
                let name = format!("motif{}", idx);
                Motif::new(&name, m).map_err(|e| MotifsError::MotifError(e))
            }) 
            .collect::<Result<Vec<Motif>, MotifsError>>()?;

        Ok(Self { motifs })
    }

    /// Searches for all motifs within the given sequence string.
    ///
    /// Finds all occurrences of any stored motif within the target sequence
    /// and returns information about their positions.
    ///
    /// # Arguments
    /// * `other` - The target sequence string to search within
    ///
    /// # Returns
    /// * `Option<Vec<ChunkInfo>>` - Vector of match information if any motifs are found,
    ///   None if no matches are found
    pub(crate) fn self_in_other(&self, other: &[u8]) -> Option<Vec<ChunkInfo>> {
        let mut hits: Vec<ChunkInfo> = Vec::new();

        for motif in &self.motifs {
            if let Some(start_indices) = motif.is_in(other) {
                for start_index in start_indices {
                    let chunk_info = ChunkInfo::new(
                        motif.name().to_string(),
                        start_index,
                        start_index + motif.len()
                    );
                    hits.push(chunk_info);
                }
            }
        }

        if hits.is_empty() {
            None
        } else {
            Some(hits)
        }
    }

    /// Checks if all motifs have the same length
    /// 
    /// # Returns 
    /// The length of the motifs is all motifs have the same length,
    /// None otherwise
    pub(crate) fn equal_len(&self) -> Option<usize> {
        let first_len = match self.motifs.get(0) {
            Some(motif) => motif.len(),
            None => return None
        };

        if self.motifs
            .iter()
            .all(|m| m.len() == first_len) 
        {
            Some(first_len)
        } else {
            None
        }
    }
}