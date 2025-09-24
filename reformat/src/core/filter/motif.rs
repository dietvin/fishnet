use crate::error::core::filter::MotifError;

/// A single sequence motif pattern for sequence matching.
///
/// Motifs are stored as uppercase strings containing only A, C, G, T characters.
/// RNA sequences (containing U) are automatically converted to DNA (U -> T).
#[derive(Debug)]
pub(crate) struct Motif {
    name: String,
    motif: String
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
        let motif = motif.to_uppercase().replace("U", "T");
        Self::is_valid_motif(&motif)?;

        Ok(Self { name: name.to_string(), motif })
    }

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
    fn is_valid_motif(motif_uppercase: &String) -> Result<(), MotifError> {
        if motif_uppercase.chars().all(|c| ['A', 'C', 'G', 'T'].contains(&c)) {
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
    pub(crate) fn is_in(&self, other: &str) -> Option<Vec<usize>> {
        if other.len() < self.motif.len() {
            return None;
        }
        let matches = other.match_indices(&self.motif).map(|(idx, _)| idx).collect::<Vec<usize>>();
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

    /// Returns the motif sequence pattern.
    ///
    /// # Returns
    /// * `&str` - Reference to the motif sequence
    pub(crate) fn motif(&self) -> &str {
        &self.motif
    }

    /// Returns the length of the motif sequence.
    ///
    /// # Returns
    /// * `usize` - Length of the motif in bases
    pub(crate) fn len(&self) -> usize {
        self.motif.len()
    }
}