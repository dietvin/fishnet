use arrow2::{array::{Array, BinaryArray, FixedSizeBinaryArray, UInt32Array}, chunk::Chunk};
use uuid::Uuid;

use crate::{error::tables::SignalTableError, core::tables::decompression::decode};

/// A single row of decompressed signal data.
/// 
/// Each row contains the read ID (as a UUID), the decompressed signal values,
/// the number of bytes used to store the compressed signal, and the total number
/// of samples after decompression.
#[derive(Debug)]
pub struct SignalTableRow {
    pub read_id: Uuid,
    pub signal: Vec<i16>,
    pub byte_count: usize,
    pub sample_count: usize
}


/// An iterator over rows of signal data stored in Arrow arrays.
/// 
/// The signal data includes compressed signal bytes, corresponding read IDs,
/// and the number of samples. Data is lazily decompressed when accessed
/// via `get()` or the iterator interface.
#[derive(Debug)]
pub(crate) struct SignalTable {
    read_id_array: FixedSizeBinaryArray,
    signal_array: BinaryArray<i64>,
    num_samples_array: UInt32Array,
    current_index: usize,
    length: usize
}

impl SignalTable {
    /// Constructs a `SignalTable` from a chunk of Arrow arrays.
    /// 
    /// Expects three columns in the following order:
    /// 1. `FixedSizeBinaryArray` for read IDs (UUIDs)
    /// 2. `BinaryArray<i64>` for compressed signal bytes
    /// 3. `UInt32Array` for the number of samples
    /// 
    /// # Errors
    /// 
    /// Returns a `SignalTableError::DowncastError` if any of the arrays
    /// cannot be downcast to their expected types.
    pub(crate) fn from_chunk(chunk: Chunk<Box<dyn Array>>) -> Result<Self, SignalTableError> {
        let arrays = chunk.arrays();

        let read_id_array = arrays[0]
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .ok_or(SignalTableError::DowncastError("read_id", "FixedSizeBinaryArray"))?
            .clone();

        let signal_array = arrays[1]
            .as_any()
            .downcast_ref::<BinaryArray<i64>>()
            .ok_or(SignalTableError::DowncastError("signal", "BinaryArray<i64>"))?
            .clone();

        let num_samples_array = arrays[2]
            .as_any()
            .downcast_ref::<UInt32Array>()
            .ok_or(SignalTableError::DowncastError("num_samples", "UInt32Array"))?
            .clone();
        let length = read_id_array.len();

        Ok(SignalTable { 
            read_id_array, 
            signal_array, 
            num_samples_array, 
            current_index: 0, 
            length
        })
    }

    /// Returns the number of entries in the table.
    pub(crate) fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if the table contains no entries.
    pub(crate) fn is_empty(&self) -> bool {
        self.length == 0
    }

    fn get_uuid(uuid_bytes: &[u8]) -> Result<Uuid, SignalTableError> {
        if uuid_bytes.len() != 16 {
            return Err(SignalTableError::InvalidUuidLength(uuid_bytes.len()));
        }
        let mut uuid_array = [0u8; 16];
        uuid_array.copy_from_slice(uuid_bytes);

        Ok(Uuid::from_bytes(uuid_array))
    }

    fn get_decompress_signal(signal_bytes: &[u8], byte_count: usize) -> Result<Vec<i16>, SignalTableError> {
        Ok(decode(signal_bytes, byte_count)?)
    }

    /// Retrieves and decompresses the signal data at the specified index.
    /// 
    /// # Arguments
    /// 
    /// * `index` - The row index to retrieve.
    /// 
    /// # Returns
    /// 
    /// A `SignalTableRow` containing the UUID, decompressed signal,
    /// compressed byte count, and sample count.
    /// 
    /// # Errors
    /// 
    /// * `SignalTableError::SignalIndexOutOfBounds` if the index is invalid.
    /// * `SignalTableError::InvalidUuidLength` if the UUID byte length is incorrect.
    /// * `SignalTableError::DecompressError` if decompression fails.
    pub fn get(&self, index: usize) -> Result<SignalTableRow, SignalTableError> {
        if index >= self.length {
            return Err(SignalTableError::SignalIndexOutOfBounds(index, self.length));
        }
        
        let read_id = Self::get_uuid(
            self.read_id_array.value(index)
        )?;

        let byte_count = self.signal_array.value(index).len();
        let sample_count = self.num_samples_array.value(index) as usize;

        let signal = Self::get_decompress_signal(
            self.signal_array.value(index),
            sample_count
        )?;

        Ok(SignalTableRow { 
            read_id, 
            signal, 
            byte_count, 
            sample_count 
        })
    }
}

/// Allows iteration over rows in the `SignalTable`, decompressing
/// each signal on-the-fly. Yields a `Result<SignalTableRow, SignalTableError>`.
impl Iterator for SignalTable {
    type Item = Result<SignalTableRow, SignalTableError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.length {
            return None;
        }
        
        let result = self.get(self.current_index);
        self.current_index += 1;
        Some(result)
    }
}