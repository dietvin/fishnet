use bam_errors::{BamFileError, BamReadError, RefSeqReconstructError};
use pod5_errors::{Pod5FileError, Pod5IndexError, Pod5ReadError};

pub mod bam_errors;
pub mod pod5_errors;
pub mod file_handling_errors;

#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    // Bam errors
    #[error("Bam read error: {0}")]
    BamReadError(#[from] BamReadError),
    #[error("Bam file error: {0}")]
    BamFileError(#[from] BamFileError),
    #[error("Bam ref seq reconstruction error: {0}")]
    BamRefSeqReconstructError(#[from] RefSeqReconstructError),

    // Pod5 errors
    #[error("Pod5 read error: {0}")]
    Pod5ReadError(#[from] Pod5ReadError),
    #[error("Pod5 file error: {0}")]
    Pod5FileError(#[from] Pod5FileError),    
    #[error("Pod5 index error: {0}")]
    Pod5IndexError(#[from] Pod5IndexError)
}
