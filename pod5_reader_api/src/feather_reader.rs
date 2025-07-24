pub mod bounded_reader;

use std::{fs::File, io::Read};

use arrow2::{
    array::Array, 
    chunk::Chunk, 
    datatypes::Schema, 
    io::ipc::read::{
        read_file_metadata, 
        FileMetadata, 
        FileReader
    } 
};

use arrow2::error::Result as ArrowResult;
use arrow2::error::Error;

use crate::feather_reader::bounded_reader::{BoundedReader, BoundedReaderError};


/// Reader for FeatherV2 datasets embedded within pod5 files.
/// Provides access to Arrow data through a bounded file reader,
/// including schema information and chunk-based data iteration.
#[derive(Debug)]
pub(crate) struct FeatherReader {
    pub embedded_reader: BoundedReader<File>,
    pub metadata: FileMetadata
}

impl FeatherReader {
    /// Creates a new `FeatherReader` instance for reading FeatherV2 datasets embedded within pod5 files.
    /// 
    /// This function initializes a bounded reader for the specified file section and validates
    /// that it contains a valid Arrow/Feather dataset by checking the magic bytes. It then
    /// reads the file metadata to prepare for data access.
    /// 
    /// # Arguments
    /// 
    /// * `file` - The file handle to read from
    /// * `offset` - The byte offset where the embedded FeatherV2 data begins
    /// * `length` - The length in bytes of the embedded FeatherV2 data
    /// 
    /// # Returns
    /// 
    /// A `Result` containing the new `FeatherReader` instance on success, or a
    /// `FeatherReaderError` if initialization fails (e.g., invalid magic bytes,
    /// corrupted metadata, or I/O errors)
    /// 
    /// # Errors
    /// 
    /// * `FeatherReaderError::Arrow2Error` - If the Arrow magic bytes are invalid or metadata is corrupted
    /// * `FeatherReaderError::IoError` - If file I/O operations fail
    /// * `FeatherReaderError::EmbeddedFeatherReaderError` - If the bounded reader initialization fails
    pub fn new(file: File, offset: i64, length: i64) -> Result<Self, FeatherReaderError> {
        // Intialize the underlying reader
        let mut embedded_reader = BoundedReader::new(
            file, 
            offset as u64, 
            length as u64
        )?;
        
        // Verify Arrow magic bytes
        let mut magic = [0u8; 6];
        embedded_reader.read_exact(&mut magic)?;
        if &magic != b"ARROW1" {
            return Err(FeatherReaderError::Arrow2Error(Error::OutOfSpec(
                format!("Invalid Arrow magic bytes: {:?}", magic)
            )));
        }
        embedded_reader.reset()?;

        // Retrieve metadata
        let metadata = read_file_metadata(&mut embedded_reader)?;

        Ok(FeatherReader { 
            embedded_reader, 
            metadata
        })  
    }

    /// Returns a reference to the Arrow schema of the FeatherV2 dataset.
    /// 
    /// The schema contains information about the column names, data types, and structure
    /// of the data stored in the dataset.
    /// 
    /// # Returns
    /// 
    /// A reference to the Arrow `Schema` object
    pub fn schema(&self) -> &Schema {
        &self.metadata.schema
    }

    /// Resets the internal reader position to the beginning of the dataset.
    /// 
    /// This allows re-reading the data from the start after previous iterations
    /// or chunk accesses have moved the read position.
    /// 
    /// # Returns
    /// 
    /// A `Result` indicating success or failure of the reset operation
    /// 
    /// # Errors
    /// 
    /// * `FeatherReaderError::EmbeddedFeatherReaderError` - If resetting the bounded reader fails
    pub fn reset(&mut self) -> Result<(), FeatherReaderError> {
        self.embedded_reader.reset()?;
        Ok(())
    }
    /// Creates an iterator for reading data chunks from the FeatherV2 dataset.
    /// 
    /// This method resets the reader to the beginning and returns an iterator that
    /// yields chunks of Arrow data. Each chunk contains a batch of records from
    /// the dataset.
    /// 
    /// # Returns
    /// 
    /// A `Result` containing a `ChunkIterator` on success, or a `FeatherReaderError`
    /// if the reset operation fails
    /// 
    /// # Errors
    /// 
    /// * `FeatherReaderError::EmbeddedFeatherReaderError` - If resetting the reader fails
    pub fn iter_chunks(&mut self) -> Result<ChunkIterator<'_>, FeatherReaderError> {
        self.reset()?;

        Ok(ChunkIterator {
            reader: FileReader::new(&mut self.embedded_reader, self.metadata.clone(), None, None)
        })
    }

    /// Retrieves a specific data chunk by its index.
    /// 
    /// **Warning**: This method is highly inefficient as it iterates through all
    /// chunks from the beginning until the target index is reached. For sequential
    /// access, prefer using `iter_chunks()` instead.
    /// 
    /// # Arguments
    /// 
    /// * `target_index` - The zero-based index of the desired chunk
    /// 
    /// # Returns
    /// 
    /// A `Result` containing the requested `Chunk` on success, or a `FeatherReaderError`
    /// if the index is out of bounds or other errors occur
    /// 
    /// # Errors
    /// 
    /// * `FeatherReaderError::IndexOutOfBounds` - If the target index exceeds the number of available chunks
    /// * `FeatherReaderError::EmbeddedFeatherReaderError` - If resetting the reader fails
    /// * `FeatherReaderError::Arrow2Error` - If reading the chunk data fails
    pub fn get_chunk(&mut self, target_index: usize) -> Result<Chunk<Box<dyn Array>>, FeatherReaderError> {
        self.reset()?;

        let reader = FileReader::new(&mut self.embedded_reader, self.metadata.clone(), None, None);

        for (current_index, chunk_result) in reader.enumerate() {
            if current_index == target_index {
                return Ok(chunk_result?);
            }
        }

        Err(FeatherReaderError::IndexOutOfBounds(target_index))
    }

    /// Returns an immutable reference to the underlying bounded reader.
    /// 
    /// This provides access to the bounded reader that handles the file I/O
    /// operations for the embedded FeatherV2 data.
    /// 
    /// # Returns
    /// 
    /// An immutable reference to the `BoundedReader<File>`
    pub fn embedded_reader(&self) -> &BoundedReader<File> {
        &self.embedded_reader
    }

    /// Returns a mutable reference to the underlying bounded reader.
    /// 
    /// This provides mutable access to the bounded reader for advanced operations
    /// that require direct manipulation of the reader state.
    /// 
    /// # Returns
    /// 
    /// A mutable reference to the `BoundedReader<File>`
    pub fn embedded_reader_mut(&mut self) -> &mut BoundedReader<File> {
        &mut self.embedded_reader
    }

    /// Returns an immutable reference to the file metadata.
    /// 
    /// The metadata contains information about the Arrow file structure,
    /// including schema, record batch locations, and other file-level details.
    /// 
    /// # Returns
    /// 
    /// An immutable reference to the `FileMetadata`
    pub fn metadata(&self) -> &FileMetadata {
        &self.metadata
    }
}


/// Iterator that yields data chunks from a FeatherV2 dataset.
/// Wraps the Arrow FileReader to provide sequential access to
/// batches of records stored in the embedded dataset.
pub struct ChunkIterator<'a> {
    reader: FileReader<&'a mut BoundedReader<File>>
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = ArrowResult<Chunk<Box<dyn Array>>>;

    /// Advances the iterator to the next chunk in the FeatherV2 dataset.
    /// 
    /// This method is part of the `Iterator` trait implementation and returns
    /// the next available data chunk, or `None` when all chunks have been consumed.
    /// 
    /// # Returns
    /// 
    /// An `Option` containing either:
    /// * `Some(Ok(Chunk))` - The next data chunk
    /// * `Some(Err(Error))` - An error occurred while reading the chunk
    /// * `None` - No more chunks are available
    fn next(&mut self) -> Option<Self::Item> {
        self.reader.next()
    }
}

/// Error types that can occur during FeatherReader operations.
/// Covers bounded reader errors, Arrow format errors, I/O failures,
/// and index out of bounds conditions.
#[derive(Debug, thiserror::Error)]
pub enum FeatherReaderError {
    #[error("Embedded feather reader error: {0}")]
    EmbeddedFeatherReaderError(#[from] BoundedReaderError),
    #[error("Arrow2 error: {0}")]
    Arrow2Error(#[from] arrow2::error::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(usize),
}