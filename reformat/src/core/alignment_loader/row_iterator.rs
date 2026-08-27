use std::{
    fs::File, 
    path::PathBuf
};
use arrow2::io::parquet::read::FileReader;
use pod5_reader_api::dataset::Pod5Dataset;
use arrow2::io::parquet::read::{
    infer_schema, 
    read_metadata
};

use crate::{
    core::alignment_loader::{
        alignment_chunk::AlignmentChunk, 
        column_index::ColumnIndex, row::Row
    }, 
    error::core::loader::RowIteratorError, 
    execute::config::Column
};

/// Iterator that provides row-by-row access to alignment data from a parquet file.
/// 
/// This iterator lazily loads chunks from the parquet file and processes them
/// row by row, optionally integrating with a Pod5 dataset for signal data.
pub(crate) struct RowIterator {
    /// Column index mapping for this file
    column_index: ColumnIndex,
    /// Optional Pod5 dataset for signal data lookup
    pod5_dataset: Option<Pod5Dataset>,
    /// Flag that indicates whether to reverse the signal in 
    /// case signal get extracted from the Pod5Dataset
    is_rna: bool,
    /// Arrow FileReader for the parquet file
    file_reader: FileReader<File>,
    /// Currently loaded chunk (None if no more chunks)
    current_chunk: AlignmentChunk,
    /// Index of the next row to return from current chunk
    current_chunk_index: usize
}


impl RowIterator {
    /// Creates a new RowIterator for the given parquet file.
    /// 
    /// # Arguments
    /// * `path` - Path to the parquet file
    /// * `chunk_size` - Number of rows to load per chunk
    /// * `columns_of_interest` - Columns that must be available
    /// * `pod5_dataset` - Optional Pod5 dataset for signal data
    /// 
    /// # Returns
    /// * `Ok(RowIterator)` - Successfully initialized iterator
    /// * `Err(RowIteratorError)` - Failed to open file or parse schema
    /// 
    /// # Behavior
    /// - Reads parquet metadata immediately but doesn't load data
    /// - Validates that required columns are present
    /// - Does not load the first chunk until iteration begins
    pub(crate) fn new(
        path: &PathBuf,
        chunk_size: usize, 
        columns_of_interest: &[Column], 
        pod5_dataset: Option<Pod5Dataset>,
        is_rna: bool
    ) -> Result<Self, RowIteratorError> {
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
            .ok_or(RowIteratorError::NoChunks)??;
        let current_chunk = AlignmentChunk::from_chunk(
            chunk, 
            &column_index
        )?;

        Ok(Self {
            column_index,
            pod5_dataset,
            is_rna,
            file_reader,
            current_chunk,
            current_chunk_index: 0,
        })
    }
}

impl Iterator for RowIterator {
    type Item = Result<Row, RowIteratorError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check if a new chunk needs to be loaded
        if self.current_chunk_index >= self.current_chunk.length {
            let chunk = match self.file_reader.next()? {
                Ok(c) => c,
                Err(e) => return Some(Err(RowIteratorError::ArrowError(e)))
            };

            self.current_chunk = match AlignmentChunk::from_chunk(chunk, &self.column_index) {
                Ok(c) => c,
                Err(e) => return Some(Err(RowIteratorError::AlignmentChunkError(e)))
            };
            self.current_chunk_index = 0;
        }

        // Try to load the next row
        match self.current_chunk.get_row(
            self.current_chunk_index, 
            &mut self.pod5_dataset,
            self.is_rna
        ) {
            Ok(row) => {
                self.current_chunk_index += 1;
                Some(Ok(row))
            },
            Err(e) => Some(Err(RowIteratorError::AlignmentChunkError(e)))
        }
    }
}