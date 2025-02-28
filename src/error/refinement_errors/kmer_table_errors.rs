#[derive(Debug, thiserror::Error)]
pub enum KmerTableError {
    #[error("IO Error: {0}")]
    FileNotFound(#[from] std::io::Error),
    #[error("File is empty")]
    EmptyFile,
    #[error("Empty kmer")]
    EmptyKmer,
    #[error("Invalid kmer: k ({0}) is even")]
    EvenKmer(usize),
    #[error("Line Parsing error: (found {0} columns, expected 2)")]
    LineParsingError(usize),
    #[error("Duplicate kmer: {0}")]
    DuplicateKmer(String),
    #[error("Kmer length ({0}) differs from rest ({1})")]
    NonUniformKmerLength(usize, usize),
    #[error("Could not convert level to f32: {0}")]
    FloatConversionError(#[from] std::num::ParseFloatError),
    #[error("Kmer table contains fewer entries than expected ({0} vs {1})")]
    MissingEntries(usize, usize)
}