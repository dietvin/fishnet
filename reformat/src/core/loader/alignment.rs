/// Parquet/JSON input wrapper for genomic alignment data processing.
/// 
/// This module provides row-wise data reading with lazy loading of metadata.
/// It supports both embedded signal data and external Pod5 dataset integration.

use std::{
    collections::HashMap, 
    fs::File, 
    path::PathBuf
};
use arrow2::{
    array::{
        Array, 
        Int16Array, 
        ListArray,
        UInt64Array,
        Utf8Array
    }, 
    chunk::Chunk, 
    datatypes::Schema, 
    io::parquet::read::{
        infer_schema, 
        read_metadata, 
        FileReader
    }
};
use pod5_reader_api::dataset::Pod5Dataset;
use uuid::Uuid;

use crate::{
    core::loader::reference_regions::ReferenceRegion, error::core::loader::{
        AlignmentChunkError, ColumnIndexError, RowError, RowIteratorError
    }, execute::config::Column
};

/// Maps column names to their indices in the parquet schema.
/// 
/// This struct provides efficient access to column data by maintaining
/// the mapping between the column types and their positions in the Arrow
/// schema.
struct ColumnIndex {
    /// Index of the read_id column (always required)
    read_id: usize,
    /// Index of the alignment column (query_to_signal or ref_to_signal)
    alignment: usize,
    /// Index of the sequence column (query_sequence or ref_sequence), if present
    sequence: Option<usize>,
    /// Index of the reference name column, if present
    ref_name: Option<usize>,
    /// Index of the reference start position column, if present
    ref_start: Option<usize>,
    /// Index of the signal data column, if embedded in parquet
    signal: Option<usize>
}

impl ColumnIndex {
    /// Creates a new ColumnIndex by analyzing the parquet schema.
    /// 
    /// # Arguments
    /// * `schema` - The Arrow schema from the parquet file
    /// * `columns_of_interest` - Vector of columns that should be available
    /// 
    /// # Returns
    /// * `Ok(ColumnIndex)` - Successfully mapped column indices
    /// * `Err(ColumnIndexError)` - Missing required columns or unexpected field names
    /// 
    /// # Column Requirements
    /// - `ReadId` is always required
    /// - Either `QueryAlignment` or `RefAlignment` must be present
    /// - If `RefName` is requested, `RefStart` must also be available
    /// - Sequences and signal data are optional depending on use case
    fn from_schema(
        schema: &Schema,
        columns_of_interest: &[Column]
    ) -> Result<Self, ColumnIndexError> {
        // Map field names to Column enum variants
        let field_columns = schema.fields.iter()
            .map(|field| {
                Ok(match field.name.as_str() {
                    "read_id" => Column::ReadId,
                    "query_to_signal" => Column::QueryAlignment,
                    "query_sequence" => Column::QuerySequence,
                    "ref_to_signal" => Column::RefAlignment,
                    "ref_sequence" => Column::RefSequence,
                    "ref_name" => Column::RefName,
                    "ref_start" => Column::RefStart,
                    "signal" => Column::Signal,
                    _ => return Err(ColumnIndexError::UnexpectedFieldName(
                        field.name.clone())
                    )
                })
            })
            .collect::<Result<Vec<Column>, ColumnIndexError>>()?;

        // Create mapping from Column to index
        let field_indices = field_columns
            .into_iter()
            .enumerate()
            .map(|(idx, col)| (col, idx))
            .collect::<HashMap<Column, usize>>();

        // Columns of interest can contain the following data:
        // - ReadId always present
        // - Always one of: QueryAlignment, RefAlignment (depending of alignment type)
        // - One of the following (depending on filter source):
        //      1. RefName and RefStart
        //      2. One of: QuerySequence, RefSequence 
        // - Optionally: Signal

        // ReadId is always required
        let read_id = *field_indices.get(&Column::ReadId)
            .ok_or_else(|| ColumnIndexError::MissingColumn("read_id", Column::QueryAlignment))?;
        
        // Determine alignment column (query or reference)
        let alignment = if columns_of_interest.contains(&Column::QueryAlignment) {
            *field_indices.get(&Column::QueryAlignment)
                .ok_or_else(|| ColumnIndexError::MissingColumn("alignment", Column::QueryAlignment))?
        } else {
            *field_indices.get(&Column::RefAlignment)
                .ok_or_else(|| ColumnIndexError::MissingColumn("alignment", Column::RefAlignment))?
        };

        // Determine sequence column
        let sequence = if columns_of_interest.contains(&Column::QuerySequence) {
            Some(*field_indices.get(&Column::QuerySequence)
                .ok_or_else(|| ColumnIndexError::MissingColumn("sequence", Column::QuerySequence))?
            )
        } 
        else if columns_of_interest.contains(&Column::RefSequence) {
            Some(*field_indices.get(&Column::RefSequence)
                .ok_or_else(|| ColumnIndexError::MissingColumn("sequence", Column::RefSequence))?
            )
        } else {
            // Try to get sequences anyway for output, but don't error if missing
            if columns_of_interest.contains(&Column::QueryAlignment) {
                field_indices.get(&Column::QuerySequence).copied()
            } else {
                field_indices.get(&Column::RefSequence).copied()
            }
        };

        // If RefName is present, RefStart must also be present
        let (ref_name, ref_start) = if columns_of_interest.contains(&Column::RefName) {
            let name = *field_indices.get(&Column::RefName)
                .ok_or_else(|| ColumnIndexError::MissingColumn("ref_name", Column::RefName))?;
            let start = *field_indices.get(&Column::RefStart)
                .ok_or_else(|| ColumnIndexError::MissingColumn("ref_start", Column::RefStart))?;
            (Some(name), Some(start))
        } else {
            (None, None)
        };

        // Signal column is optional
        let signal = if columns_of_interest.contains(&Column::Signal) {
            Some(*field_indices.get(&Column::Signal)
                .ok_or(ColumnIndexError::MissingColumn("signal", Column::Signal))?
            )
        } else {
            None
        };

        Ok(Self { 
            read_id,
            alignment,
            sequence,
            ref_name,
            ref_start,
            signal 
        })
    }
}


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
    sequence: String,
    /// Reference sequence name this read aligns to (if applicable)
    ref_region: Option<ReferenceRegion>,
    /// Raw current measurements
    signal: Vec<i16>
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
    fn new(
        read_id: Uuid,
        alignment: Vec<usize>,
        sequence: String,
        signal: Vec<i16>,
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
    pub(crate) fn sequence(&self) -> &str {
        &self.sequence
    }

    /// Returns the reference region if available.
    pub(crate) fn ref_region(&self) -> Option<&ReferenceRegion> {
        self.ref_region.as_ref()
    }

    /// Returns the raw signal data.
    pub(crate) fn signal(&self) -> &[i16] {
        &self.signal
    }
}


/// Represents a chunk of alignment data loaded from parquet.
/// 
/// This struct holds vectorized data for multiple rows, enabling efficient
/// batch processing while maintaining the ability to extract individual rows.
struct AlignmentChunk {
    /// Number of rows in this chunk
    length: usize, 
    /// Vector of read IDs for all rows
    read_id: Vec<Uuid>,
    /// Vector of alignment coordinate vectors
    alignment: Vec<Vec<usize>>,
    /// Optional vector of sequence strings
    sequences: Option<Vec<String>>,
    /// Optional vector of reference names
    ref_name: Option<Vec<String>>,
    /// Optional vector of reference start positions
    ref_start: Option<Vec<usize>>,
    /// Optional vector of signal data vectors
    signal: Option<Vec<Vec<i16>>>
}

impl AlignmentChunk {
    /// Creates an AlignmentChunk from an Arrow chunk using the provided column mapping.
    /// 
    /// # Arguments
    /// * `chunk` - Arrow chunk containing the raw columnar data
    /// * `column_index` - Mapping from semantic columns to physical indices
    /// 
    /// # Returns
    /// * `Ok(AlignmentChunk)` - Successfully parsed chunk
    /// * `Err(AlignmentChunkError)` - Failed to parse data or missing columns
    fn from_chunk(
        chunk: Chunk<Box<dyn Array>>, 
        column_index: &ColumnIndex
    ) -> Result<Self, AlignmentChunkError> {
        let arrays = chunk.arrays();

        let read_id = Self::parse_read_id_col(
            arrays.get(column_index.read_id)
            .ok_or_else(|| AlignmentChunkError::ColumnIndexError(
                "read_id", column_index.read_id
            ))?
        )?;

        let alignment = Self::parse_alignment_col(
            arrays.get(column_index.alignment)
            .ok_or_else(|| AlignmentChunkError::ColumnIndexError(
                "alignment", column_index.alignment
            ))?
        )?;

        let sequences = column_index.sequence
            .map(|idx| {
                Self::parse_string_col(
                    arrays.get(idx)
                        .ok_or_else(|| AlignmentChunkError::ColumnIndexError(
                            "sequence", idx
                        ))?
                )
            })
            .transpose()?;

        let ref_name = column_index.ref_name
            .map(|idx| {
                Self::parse_string_col(
                    arrays.get(idx)
                        .ok_or_else(|| AlignmentChunkError::ColumnIndexError(
                            "ref_name", idx
                        ))?
                )
            })
            .transpose()?;

        let ref_start = column_index.ref_start
            .map(|idx| {
                Self::parse_usize_col(
                    arrays.get(idx)
                        .ok_or_else(|| AlignmentChunkError::ColumnIndexError(
                            "ref_start", idx
                        ))?
                )
            })
            .transpose()?;

        let signal = column_index.signal
            .map(|idx| {
                Self::parse_signal_col(
                    arrays.get(idx)
                        .ok_or_else(|| AlignmentChunkError::ColumnIndexError(
                            "signal", idx
                        ))?
                )
            })
            .transpose()?;

        Ok(Self { 
            length: read_id.len(),
            read_id,
            alignment,
            sequences,
            ref_name,
            ref_start,
            signal
        })
    }

    /// Parses a column containing UUID strings.
    fn parse_read_id_col(array: &Box<dyn Array>) -> Result<Vec<Uuid>, AlignmentChunkError> {
        array
            .as_any()
            .downcast_ref::<Utf8Array<i32>>()
            .ok_or_else(|| AlignmentChunkError::DowncastError("Utf8Array<i32>"))?
            .iter()
            .map(|el_opt| {
                el_opt
                    .ok_or(AlignmentChunkError::ValueNone)?
                    .parse::<Uuid>()
                    .map_err(AlignmentChunkError::UuidError)
            })
            .collect()
    }

    /// Parses a column containing lists of alignment coordinates.
    fn parse_alignment_col(array: &Box<dyn Array>) -> Result<Vec<Vec<usize>>, AlignmentChunkError> {
        array
            .as_any()
            .downcast_ref::<ListArray<i32>>()
            .ok_or_else(|| AlignmentChunkError::DowncastError("ListArray<i32>"))?
            .iter()
            .map(|arr_opt| {
                let arr = arr_opt.ok_or(AlignmentChunkError::ValueNone)?;
                Self::parse_usize_col(&arr)
            })
            .collect()
    }

    /// Parses a column containing string values.
    fn parse_string_col(array: &Box<dyn Array>) -> Result<Vec<String>, AlignmentChunkError> {
        array
            .as_any()
            .downcast_ref::<Utf8Array<i32>>()
            .ok_or_else(|| AlignmentChunkError::DowncastError("Utf8Array<i32>"))?
            .iter()
            .map(|el_opt| {
                el_opt
                    .ok_or(AlignmentChunkError::ValueNone)
                    .map(|s| s.to_string())
            })
            .collect()
    }

    /// Parses a column containing usize values (stored as UInt64).
    fn parse_usize_col(array: &Box<dyn Array>) -> Result<Vec<usize>, AlignmentChunkError> {
        array
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or_else(|| AlignmentChunkError::DowncastError("UInt64Array"))?
            .iter()
            .map(|el_opt| {
                el_opt
                    .ok_or(AlignmentChunkError::ValueNone)
                    .map(|&val| val as usize)
            })
            .collect()   
    }

    /// Parses a column containing lists of i16 signal values.
    fn parse_signal_col(array: &Box<dyn Array>) -> Result<Vec<Vec<i16>>, AlignmentChunkError> {
        array
            .as_any()
            .downcast_ref::<ListArray<i32>>()
            .ok_or_else(|| AlignmentChunkError::DowncastError("ListArray<i32>"))?
            .iter()
            .map(|arr_opt| {
                let arr = arr_opt.ok_or(AlignmentChunkError::ValueNone)?;
                arr.as_any()
                        .downcast_ref::<Int16Array>()
                        .ok_or(AlignmentChunkError::DowncastError("UInt16Array"))?
                        .iter().map(|el_opt| {
                            el_opt
                                .copied()
                                .ok_or(AlignmentChunkError::ValueNone)
                        })
                        .collect()
            })
            .collect()
    }

    /// Extracts a single row from this chunk.
    /// 
    /// # Arguments
    /// * `idx` - Index of the row to extract (must be < length)
    /// * `pod5_dataset` - Optional Pod5 dataset for signal data lookup
    /// 
    /// # Returns
    /// * `Ok(Row)` - Successfully extracted row
    /// * `Err(AlignmentChunkError)` - Invalid index or failed to get signal data
    /// 
    /// # Behavior
    /// - If signal data is embedded in parquet, uses that
    /// - If signal data is missing and pod5_dataset is available, fetches from Pod5
    /// - If sequence data is missing, generates N-filled placeholder
    fn get_row(&mut self, idx: usize, pod5_dataset: &mut Option<Pod5Dataset>) -> Result<Row, AlignmentChunkError> {
        if idx >= self.length {
            return Err(AlignmentChunkError::InvalidIndex(idx, self.length));
        }

        let read_id = self.read_id[idx];
        let alignment = self.alignment[idx].clone();

        let sequence = match &self.sequences {
            Some(seq) => seq[idx].clone(),
            None => {
                let seq_len = alignment.len().saturating_sub(1).max(1);
                "N".repeat(seq_len).to_string()
            }
        };

        let ref_name = self.ref_name
            .as_ref()
            .map(|names| names[idx].clone());
    
        let ref_start = self.ref_start
            .as_ref()
            .map(|names| names[idx]);

        let signal = match &self.signal {
            Some(signal) => signal[idx].clone(),
            None => {
                let dataset = pod5_dataset.as_mut()
                    .ok_or(AlignmentChunkError::Pod5DatasetMissing)?;

                dataset
                    .get_read(&read_id)?
                    .require_signal()?
                    .to_vec()
            }
        };

        let row = Row::new(
            read_id, 
            alignment, 
            sequence, 
            signal, 
            ref_name, 
            ref_start
        )?;
        Ok(row)
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
        pod5_dataset: Option<Pod5Dataset>
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
            &mut self.pod5_dataset
        ) {
            Ok(row) => {
                self.current_chunk_index += 1;
                Some(Ok(row))
            },
            Err(e) => Some(Err(RowIteratorError::AlignmentChunkError(e)))
        }
    }
}