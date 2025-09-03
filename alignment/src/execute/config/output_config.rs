use crate::execute::config::WhichToAlign;

#[derive(Debug, Clone, PartialEq)]
pub struct OutputConfig {
    pub alignment_type: WhichToAlign,
    pub include_sequences: bool,
    pub include_signal: bool 
}

impl OutputConfig {
    pub fn new(
        alignment_type: WhichToAlign,
        include_sequences: bool,
        include_signal: bool
    ) -> Self {
        OutputConfig { 
            alignment_type,
            include_sequences,
            include_signal
        }
    }

    pub fn alignment_type(&self) -> &WhichToAlign {
        &self.alignment_type
    }

    pub fn include_sequences(&self) -> bool {
        self.include_sequences
    }

    pub fn include_signal(&self) -> bool {
        self.include_signal
    }

    pub fn which_to_include(&self) -> (&bool, &bool) {
        (&self.include_sequences, &self.include_signal)
    }
}