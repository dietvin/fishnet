use std::{fs::File, path::PathBuf};

use arrow2::io::parquet::read::{infer_schema, read_metadata, FileReader};

use crate::{
    core::alignment_loader::{
        alignment_chunk::AlignmentChunk, 
        column_index::ColumnIndex, 
        raw_row_data::RawRowData
    }, 
    error::core::loader::RawRowIteratorError, 
    execute::config::Column
};

/// Iterator that provides row-by-row access to raw alignment data from a parquet file.
///
/// This iterator lazily loads chunks from the parquet file and yields minimally
/// processed [`RawRowData`] structs. Unlike [`RowIterator`](crate::core::alignment_loader::row_iterator::RowIterator), 
/// this does not perform expensive Row construction, making it suitable for 
/// multi-threaded processing where construction happens in worker threads.
///
/// The iterator maintains a single chunk in memory at a time, automatically
/// loading new chunks as iteration progresses.
pub(crate) struct RawRowIterator {
    column_index: ColumnIndex,
    file_reader: FileReader<File>,
    current_chunk: AlignmentChunk,
    current_chunk_index: usize
}

impl RawRowIterator {
    /// Creates a new [`RawRowIterator`] for the given parquet file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the parquet file containing alignment data
    /// * `chunk_size` - Number of rows to load per chunk
    /// * `columns_of_interest` - Subset of columns to extract from the parquet file
    ///
    /// # Errors
    ///
    /// Returns [`RawRowIteratorError`] if:
    /// - File cannot be opened
    /// - Parquet metadata cannot be read
    /// - Schema inference fails
    /// - Column index construction fails
    /// - First chunk cannot be loaded
    pub fn new(
        path: &PathBuf,
        chunk_size: usize,
        columns_of_interest: &[Column]
    ) -> Result<Self, RawRowIteratorError> {
        let mut file = File::open(path)?;

        let metadata = read_metadata(&mut file)?;
        let schema = infer_schema(&metadata)?;

        let column_index = ColumnIndex::from_schema(&schema, columns_of_interest)?;

        let mut file_reader = FileReader::new(
            file, 
            metadata.row_groups, 
            schema, 
            Some(chunk_size), 
            None, 
            None
        );

        let chunk = file_reader.next()
            .ok_or(RawRowIteratorError::NoChunks)??;
        let current_chunk = AlignmentChunk::from_chunk(
            chunk, 
            &column_index
        )?;

        Ok(Self { 
            column_index, 
            file_reader, 
            current_chunk, 
            current_chunk_index: 0
        })

    }
}

impl Iterator for RawRowIterator {
    type Item = Result<RawRowData, RawRowIteratorError>;

    /// Advances the iterator and returns the next [`RawRowData`].
    ///
    /// When the current chunk is exhausted, automatically loads the next chunk
    /// from the parquet file. Returns `None` when all chunks have been processed.
    ///
    /// # Returns
    ///
    /// - `Some(Ok(RawRowData))` - Successfully extracted raw row data
    /// - `Some(Err(e))` - Error occurred during chunk loading or row extraction
    /// - `None` - No more rows available
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_chunk_index >= self.current_chunk.length {
            let chunk = match self.file_reader.next()? {
                Ok(c) => c,
                Err(e) => return Some(Err(RawRowIteratorError::ArrowError(e)))
            };

            self.current_chunk = match AlignmentChunk::from_chunk(chunk, &self.column_index) {
                Ok(c) => c,
                Err(e) => return Some(Err(RawRowIteratorError::AlignmentChunkError(e)))
            };
            self.current_chunk_index = 0;
        }

        match self.current_chunk.get_raw_row(
            self.current_chunk_index
        ) {
            Ok(row) => {
                self.current_chunk_index += 1;
                Some(Ok(row))
            },
            Err(e) => Some(Err(RawRowIteratorError::AlignmentChunkError(e)))
        }
    }
}