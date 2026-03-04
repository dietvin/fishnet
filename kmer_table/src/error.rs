#[derive(Debug, thiserror::Error)]
pub enum KmerTableError {
    #[error("KmerTableData error: {0}")]
    KmerTableDataError(#[from] KmerTableDataError),

    #[error("Binary k-mer error: {0}")]
    BinaryKmerError(#[from] BinaryKmerError),

    #[error("Invalid kmer length: {0} (expected {1})")]
    InvalidKmerLen(usize, usize),

    #[error("Kmer not found: {0}")]
    IndexError(String),

    #[error("Provided sequence is too short: {0} (must be at least k={1})")]
    InvalidSeqLen(usize, usize),
}

#[derive(Debug, thiserror::Error)]
pub enum KmerTableDataError {
    /// Occurs when the input file cannot be found or read
    #[error("IO Error: {0}")]
    FileNotFound(#[from] std::io::Error),

    /// Occurs when a k-mer has a different length than previously seen k-mers
    /// 
    /// The first parameter is the length of the problematic k-mer,
    /// the second parameter is the expected length
    #[error("Kmer length ({0}) differs from rest ({1})")]
    NonUniformKmerLength(usize, usize),

    /// Occurs when the same k-mer appears multiple times in the input file
    /// 
    /// The parameter is the duplicate k-mer string
    #[error("Duplicate kmer: {0}")]
    DuplicateKmer(String),

    /// Occurs when the input file exists but contains no data
    #[error("File is empty")]
    EmptyFile,

    /// Occurs when the number of entries in the table is less than expected
    /// 
    /// The first parameter is the actual number of entries,
    /// the second parameter is the expected number of entries (4^k)
    #[error("Kmer table contains fewer entries than expected ({0} vs {1})")]
    MissingEntries(usize, usize),

    /// Occurs when a line in the file doesn't have exactly 2 columns
    /// 
    /// The parameter is the number of columns found in the line
    #[error("Line Parsing error: (found {0} columns, expected 2)")]
    LineParsingError(usize),

    /// Occurs when a k-mer in the file is an empty string
    #[error("Empty kmer")]
    EmptyKmer,

    /// Occurs when a k-mer has even length but odd length is expected
    /// 
    /// The parameter is the length of the k-mer that caused the error
    #[error("Invalid kmer: k ({0}) is even")]
    EvenKmer(usize),

    /// Occurs when there is an error handling the binary representiation
    /// of a k-mer.
    /// 
    /// The parameter is the underlying BinaryKmerError
    #[error("Binary k-mer error: {0}")]
    BinaryKmerError(#[from] BinaryKmerError),

    /// Occurs when a level value cannot be parsed as a floating-point number
    /// 
    /// The parameter is the underlying parsing error
    #[error("Could not convert level to f32: {0}")]
    FloatConversionError(#[from] std::num::ParseFloatError),

    /// Occurs when the argmax function returns none
    #[error("Could not determine the index of the maximum value")]
    ArgMaxError,

    /// Occurs when the standardization in the fix_gauge function fails.
    /// 
    /// The parameter provides 
    #[error("Standardization error: {0}")]
    FixGaugeError(#[from] FixGaugeError),

}

#[derive(Debug, thiserror::Error)]
pub enum FixGaugeError {
    #[error("Could not calculate the median level")]
    MedianNone,
    #[error("Could not determine the mean absolute deviation (MAD)")]
    MadNone,
    #[error("Encountered zero division")]
    ZeroDivision
}

#[derive(Debug, thiserror::Error)]
pub enum BinaryKmerError {
    /// Occurs when k exceeds 32 (This is the max.
    /// number of nucleotides that can be encoded
    /// in a u64)
    /// 
    /// The parameter is the given k
    #[error("Invalid k: {0} (max. length is 32)")]
    InvalidKmerLen(usize),

    /// Occurs when a nucleotide is other than
    /// A, C, G, or T/U
    /// 
    /// The parameter is the nucleotide at hand
    #[error("Invalid nucleotide: {0}")]
    InvalidBaseChar(char),

    /// Occurs when a position is out of bounds 
    /// when accessing a nucleotide.
    /// 
    /// The first parameter is the given position,
    /// the second paramter is k
    #[error("Position out of bounds: {0} (k={1})")]
    PositionIndexError(usize, usize)
}