use crate::error::core::filter::MotifError;

/// A single sequence motif pattern for sequence matching.
///
/// Motifs are stored as vectors of uppercase ASCII characters containing 
/// only A, C, G, T characters. RNA sequences (containing U) are automatically 
/// converted to DNA (U -> T).
#[derive(Debug)]
pub(crate) struct Motif {
    name: String,
    motif: Vec<u8>
}

impl Motif {
    /// Creates a new motif with the given name and sequence pattern.
    ///
    /// The motif sequence is normalized to uppercase and U characters are
    /// converted to T. The sequence is validated to contain only A, C, G, T.
    ///
    /// # Arguments
    /// * `name` - Name/identifier for this motif
    /// * `motif` - The motif sequence pattern
    ///
    /// # Returns
    /// * `Result<Self, MotifError>` - The constructed Motif instance or an error
    ///
    /// # Errors
    /// Returns an error if the motif contains invalid characters.
    pub(crate) fn new(name: &str, motif: &str) -> Result<Self, MotifError> {
        let mut motif_bytes = motif.as_bytes().to_vec();
        motif_bytes.iter_mut().for_each(|c| {
            *c = c.to_ascii_uppercase();
            if *c == b'U' {
                *c = b'T';
            }
        });
        Self::is_valid_motif(&motif_bytes)?;

        Ok(Self { name: name.to_string(), motif: motif_bytes })
    }

    /// Validates a given motif
    ///
    /// Checks if the given motif contains only A, C, G and T ASCII characters
    ///
    /// # Arguments
    /// * `motif` - The ASCII vector encoding the motif
    ///
    /// # Returns
    /// * `Result<(), MotifError>` - Ok if the motif is valid
    ///
    /// # Errors
    /// Returns an error if the motif contains chars other that A, C, G or T.
    fn is_valid_motif(motif: &Vec<u8>) -> Result<(), MotifError> {
        if motif.iter().all(|&c| matches!(c, b'A' | b'C' | b'G' | b'T')) {
            Ok(())
        } else {
            Err(MotifError::InvalidChars)
        }
    }

    /// Searches for this motif within the given sequence string.
    ///
    /// Finds all starting positions where this motif occurs in the target sequence.
    ///
    /// # Arguments
    /// * `other` - The target sequence string to search within
    ///
    /// # Returns
    /// * `Option<Vec<usize>>` - Vector of starting positions if matches found,
    ///   None if no matches or target is too short
    pub(crate) fn is_in(&self, other: &[u8]) -> Option<Vec<usize>> {
        if other.len() < self.motif.len() {
            return None;
        }

        let matches = other
            .windows(self.motif.len())
            .enumerate()
            .filter_map(|(i, window)| {
                if window == self.motif.as_slice() {
                    Some(i)
                } else {
                    None
                }
            })
            .collect::<Vec<usize>>();

        if matches.is_empty() {
            None
        } else {
            Some(matches)
        }
    }

    /// Returns the name of this motif.
    ///
    /// # Returns
    /// * `&str` - Reference to the motif name
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    // /// Returns the motif sequence pattern.
    // ///
    // /// # Returns
    // /// * `&str` - Reference to the motif sequence
    // pub(crate) fn motif(&self) -> &[u8] {
    //     &self.motif
    // }

    /// Returns the length of the motif sequence.
    ///
    /// # Returns
    /// * `usize` - Length of the motif in bases
    pub(crate) fn len(&self) -> usize {
        self.motif.len()
    }
}