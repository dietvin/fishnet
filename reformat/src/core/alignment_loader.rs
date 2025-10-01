/// Parquet/JSON input wrapper for genomic alignment data processing.
/// 
/// This module provides row-wise data reading with lazy loading of metadata.
/// It supports both embedded signal data and external Pod5 dataset integration.

mod alignment_chunk;
mod column_index;

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
use uuid::Uuid;

use crate::{
    core::{
        alignment_loader::{
            alignment_chunk::AlignmentChunk, 
            column_index::ColumnIndex
        }, 
        filter::reference_region::ReferenceRegion
    }, 
    error::core::loader::{
        RowError, 
        RowIteratorError
    }, 
    execute::config::Column
};



/// Represents a single row of alignment data.
/// 
/// Contains all the information for one sequencing read including its alignment
/// to a reference, sequence data, and raw signal values.
pub(crate) struct Row {
    /// Unique identifier for this sequencing read
    read_id: Uuid,
    /// Alignment coordinates mapping query to signal or reference to signal
    alignment: Vec<usize>,
    /// DNA/RNA sequence (or N-filled placeholder if not available)
    sequence: Vec<u8>,
    /// Reference sequence name this read aligns to (if applicable)
    ref_region: Option<ReferenceRegion>,
    /// Z-standardized raw current measurements
    signal: Vec<f64>
}


impl Row {
    /// Creates a new Row with the provided data.
    /// 
    /// # Arguments
    /// * `read_id` - Unique identifier for the read
    /// * `alignment` - Vector of alignment coordinates
    /// * `sequence` - DNA/RNA sequence string
    /// * `signal` - Raw signal data
    /// * `ref_name` - Optional reference name
    /// * `ref_start` - Optional reference start position
    pub(super) fn new(
        read_id: Uuid,
        alignment: Vec<usize>,
        sequence: Vec<u8>,
        signal: Vec<f64>,
        ref_name: Option<String>,
        ref_start: Option<usize> 
    ) -> Result<Self, RowError> {
        let ref_region = match (ref_name, ref_start) {
            (Some(name), Some(start)) => Some(ReferenceRegion::from_start_and_length(
                name, 
                start, 
                sequence.len()
            )?),
            _ => None
        };

        Ok(Self {
            read_id,
            alignment,
            sequence,
            ref_region,
            signal
        })
    }

    /// Returns the read ID.
    pub(crate) fn read_id(&self) -> &Uuid {
        &self.read_id
    }

    /// Returns the alignment coordinates.
    pub(crate) fn alignment(&self) -> &[usize] {
        &self.alignment
    }

    /// Returns the sequence string.
    pub(crate) fn sequence(&self) -> &[u8] {
        &self.sequence
    }

    /// Returns the reference region if available.
    pub(crate) fn ref_region(&self) -> Option<&ReferenceRegion> {
        self.ref_region.as_ref()
    }

    /// Returns the raw signal data.
    pub(crate) fn signal(&self) -> &[f64] {
        &self.signal
    }
}



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
    /// Flag that indicates whether the signal should get normalized
    norm_signal: bool,
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
        is_rna: bool,
        norm_signal: bool
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
            norm_signal,
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
            self.is_rna,
            self.norm_signal
        ) {
            Ok(row) => {
                self.current_chunk_index += 1;
                Some(Ok(row))
            },
            Err(e) => Some(Err(RowIteratorError::AlignmentChunkError(e)))
        }
    }
}