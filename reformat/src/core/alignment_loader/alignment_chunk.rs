use arrow2::{
    array::{
        Array, 
        Int16Array, 
        ListArray,
        UInt64Array,
        Utf8Array
    }, 
    chunk::Chunk
};
use pod5_reader_api::dataset::Pod5Dataset;
use uuid::Uuid;

use crate::{
    core::alignment_loader::{ColumnIndex, Row}, 
    error::core::loader::AlignmentChunkError
};


/// Represents a chunk of alignment data loaded from parquet.
/// 
/// This struct holds vectorized data for multiple rows, enabling efficient
/// batch processing while maintaining the ability to extract individual rows.
pub(super) struct AlignmentChunk {
    /// Number of rows in this chunk
    pub(super) length: usize, 
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
    pub(super) fn from_chunk(
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
    pub(super) fn get_row(&mut self, idx: usize, pod5_dataset: &mut Option<Pod5Dataset>, is_rna: bool) -> Result<Row, AlignmentChunkError> {
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

                let mut signal = dataset
                    .get_read(&read_id)?
                    .require_signal()?
                    .to_vec();

                if is_rna {
                    signal.reverse();
                }

                signal
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
