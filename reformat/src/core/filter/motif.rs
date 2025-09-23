use crate::error::core::filter::MotifError;

pub(crate) struct Motif {
    name: String,
    motif: String
}

impl Motif {
    pub(crate) fn new(name: &str, motif: &str) -> Result<Self, MotifError> {
        let motif = motif.to_uppercase().replace("U", "T");
        Self::is_valid_motif(&motif)?;

        Ok(Self { name: name.to_string(), motif })
    }

    fn is_valid_motif(motif_uppercase: &String) -> Result<(), MotifError> {
        if motif_uppercase.chars().all(|c| ['A', 'C', 'G', 'T'].contains(&c)) {
            Ok(())
        } else {
            Err(MotifError::InvalidChars)
        }
    }

    pub(crate) fn is_in(&self, other: &str) -> bool {
        if other.len() < self.motif.len() {
            return false;
        }

        other.contains(&self.motif)
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn motif(&self) -> &str {
        &self.motif
    }
}