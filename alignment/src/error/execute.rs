use crossbeam::channel::SendError;
use pod5_reader_api::error::file::Pod5FileError;

use crate::{error::{
    core::AlignmentCoreError, 
    output::{BufferError, OutputRecordError, WriterError}
}};


#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Worker pipeline error: {0}")]
    WorkerPipelineError(#[from] WorkerPipelineError),
    #[error("Worker pipeline error: {0}")]
    WriterPipelineError(#[from] WriterPipelineError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}


#[derive(Debug, thiserror::Error)]
pub enum ProducerPipelineError {
    #[error("Pod5 file error: {0}")]
    Pod5FileError(#[from] Pod5FileError),
}


#[derive(Debug, thiserror::Error)]
pub enum WorkerPipelineError {
    #[error("Alignment core error: {0}")]
    AlignmentCoreError(#[from] AlignmentCoreError),
    #[error("OutputRecord error: {0}")]
    OutputRecordError(#[from] OutputRecordError),
    #[error("Output buffer error: {0}")]
    BufferError(#[from] BufferError),
    #[error("Results queue sender error: {0}")]
    SenderError(String),
}

impl<T> From<SendError<T>> for WorkerPipelineError {
    fn from(e: SendError<T>) -> Self {
        WorkerPipelineError::SenderError(e.to_string())
    }
}


#[derive(Debug, thiserror::Error)]
pub enum WriterPipelineError {
    #[error("Writer error: {0}")]
    WriterError(#[from] WriterError)
}


#[derive(Debug, thiserror::Error)]
pub enum KmerTableLoadingError {
    #[error("Found varying basecall model names in BAM header: {0} vs {1}")]
    InconsistentBasecallModel(String, String),

    #[error("No basecalling model found in BAM header")]
    BasecallModelNotFound,

    #[error("Could not assign a stored model to basecall model: {0}")]
    UnfittingBasecallModel(String),

    #[error("Failed to deserialize kmer table: {0}")]
    DeserializationError(#[from] Box<bincode::ErrorKind>)
}