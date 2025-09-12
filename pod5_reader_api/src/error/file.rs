use std::io::SeekFrom;

use uuid::Uuid;

use crate::{
    error::{
        feather_reader::FeatherReaderError, 
        read::Pod5ReadError, 
        tables::{
            ReadsTableError, 
            RunInfoError, 
            SignalTableError
        }, 
        footer::Pod5FooterError
    }
};

/// Enum representing all possible errors that can occur when working with a Pod5File.
#[derive(Debug, thiserror::Error)]
pub enum Pod5FileError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid signature: {0:?} (start: {1:?})")]
    InvalidSignature(Vec<u8>, SeekFrom),
    #[error("Footer error: {0}")]
    FooterError(#[from] Pod5FooterError),
    #[error("Feather reader error: {0}")]
    EmbeddedFileError(#[from] FeatherReaderError),
    #[error("Arrow2 error: {0}")]
    Arrow2Error(#[from] arrow2::error::Error),
    #[error("Run info error: {0}")]
    RunInfoError(#[from] RunInfoError),
    #[error("Pod5Read error: {0}")]
    ReadError(#[from] Pod5ReadError),
    #[error("Reads table error: {0}")]
    ReadsTableError(#[from] ReadsTableError),
    #[error("Signal table error: {0}")]
    SignalTableError(#[from] SignalTableError),
    #[error("Read not found: {0}")]
    ReadNotFound(Uuid),
    #[error("Id mismatch for signal chunk: {0} vs. {1} (expected)")]
    SignalReconstructIdError(Uuid, Uuid),
    #[error("Signal length mismatch: {0} vs. {1} (expected)")]
    SignalReconstructLengthError(usize, usize),
    #[error("Read iterator error: {0}")]
    ReadIteratorError(#[from] ReadIteratorError),
    #[error("Signal table index error: {0}")]
    SignalTableIndexError(#[from] SignalTableIndexError),
    #[error("Could not determine the chunk size for the signal table")]
    SignalTableChunkSizeError,
    
    #[error("Feather reader pool error: {0}")]
    FeatherReaderPoolError(#[from] FeatherReaderPoolError),
}

#[derive(Debug, thiserror::Error)]
pub enum SignalTableIndexError {
    #[error("Invalid index '{0}' with row_to_id index of length {1}")]
    InvalidIndex(usize, usize),
    #[error("Signal row {0} does not have a read match (row_to_id is None)")]
    RowNotAssignable(usize),
    #[error("All signal chunks were already processed for read {0}")]
    ReadAlreadyFinished(Uuid),
    #[error("Read id '{0}' not found in remaining_chunks")]
    ReadIdNotFound(Uuid),
    #[error("Not all reads were fully processed. {0} reads are missing chunk(s)")]
    NotAllReadsFinished(usize)
}

/// Enum representing all possible errors that can occur during read iteration.
#[derive(Debug, thiserror::Error)]
pub enum ReadIteratorError {
    #[error("Could not get chunk iterator: {0}")]
    ChunkIteratorError(#[from] FeatherReaderError),
    #[error("Arrow2 error: {0}")]
    Arrow2Error(#[from] arrow2::error::Error),
    #[error("No chunks found in iterator")]
    EmptyIterator,
    #[error("Signal table error: {0}")]
    SignalTableError(#[from] SignalTableError),
    #[error("Read '{0}' was not found in the read index")]
    ReadNotFoundInIndex(Uuid),
    #[error("Pod5 read error: {0}")]
    Pod5ReadError(#[from] Pod5ReadError),
    #[error("Discordant signal lengths: {0} vs {1} (expected)")]
    DiscordantSignalLength(usize, usize),
    #[error("Expected signal length not set in read")]
    ExpectedSignalLenNotFound,
    #[error("Signal table is none")]
    SignalTableNone,
    #[error("Signal table index error: {0}")]
    SignalTableIndexError(#[from] SignalTableIndexError),
    #[error("Read id from signal table row ({0}) does not match read id from index ({1})")]
    ReadIdMismatch(Uuid, Uuid),
    #[error("Invalid signal chunk index {0} (must be smaller than {1})")]
    InvalidSignalChunkIndex(usize, usize),
    #[error("Read id {0} not found in incomplete reads")]
    IncompleteReadsEntryNotFound(Uuid),
    #[error("Failed to construct full signal for read {0}. Chunk {1} is missing")]
    ConstructingIncompleteSignal(Uuid, usize),
    #[error("Not all reads were finished while parsing the signal table ({0} unfinished reads remain)")]
    IncompleteReadsAfterFinish(usize)
}


#[derive(Debug, thiserror::Error)]
pub enum FeatherReaderPoolError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("FeatherReaderError: {0}")]
    FeatherReaderError(#[from] FeatherReaderError),
    #[error("Deque is full, could not add Reader. Deque size: {0}")]
    DequeFull(usize),
    #[error("Deque is empty, could not get reader")]
    DequeEmpty,
    #[error("Arrow2 error: {0}")]
    Arrow2Error(#[from] arrow2::error::Error),
    #[error("Could not determine the chunk size for the signal table")]
    SignalTableChunkSizeError,
}
