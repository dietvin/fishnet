use std::mem::replace;

use arrow2::{
    array::{
        Array, 
        MutableArray, 
        MutableListArray, 
        MutablePrimitiveArray, 
        MutableUtf8Array, 
        TryPush
    },
    chunk::Chunk,
};

use crate::{
    error::output::BufferError,
    output::{
        buffer::Buffer,
        record::OutputRecord,
        schema::OutputSchema
    }
};

/// A columnar in-memory buffer for batching `OutputRecord`s into Arrow arrays
/// for Parquet serialization.
///
/// This buffer accumulates records in Arrow2 `MutableArray`s and flushes them
/// into a `Chunk<Box<dyn Array>>` once a configurable memory threshold is exceeded.
///
/// # Design
///
/// - Uses a struct-of-arrays layout for efficient columnar writes.
/// - Field inclusion is controlled at compile time via `OutputSchema`.
/// - Memory usage is approximated incrementally to trigger flushing.
/// - Buffers are reinitialized with preallocated capacity after each flush,
///   minmizing repeated allocations.
///
/// # Memory Model
///
/// The buffer tracks approximate memory usage based on:
/// - UTF-8 byte lengths for string fields
/// - element counts for list/primitive arrays
///
/// This estimate prioritizes dominant contributors (signal, mappings) and
/// should be sufficient for controlling batch size.
///
/// # Performance Notes
///
/// - Preallocation significantly reduces allocator overhead.
/// - `replace` is used during flush to preserve allocation patterns.
/// - Iterator-based list insertion is used for simplicity; further optimization
///   is possible via manual offset construction if needed.
///
/// # Invariants
///
/// - Presence of fields in `OutputRecord` must match `OutputSchema`.
/// - All pushed columns maintain equal row counts.
/// - Schema used by the writer must match the schema implied by `OutputSchema`.
#[derive(Clone)]
pub struct ParquetBuffer {
    read_id: MutableUtf8Array<i32>,
    query_to_sig: MutableListArray<i32, MutablePrimitiveArray<u64>>,
    query_shift: MutablePrimitiveArray<f32>,
    query_scale: MutablePrimitiveArray<f32>,
    ref_to_sig: MutableListArray<i32, MutablePrimitiveArray<u64>>,
    ref_shift: MutablePrimitiveArray<f32>,
    ref_scale: MutablePrimitiveArray<f32>,
    ref_name: MutableUtf8Array<i32>,
    ref_start: MutablePrimitiveArray<u64>,
    query_seq: MutableUtf8Array<i32>,
    ref_seq: MutableUtf8Array<i32>,
    signal: MutableListArray<i32, MutablePrimitiveArray<f32>>,

    current_size_bytes: usize,
    flush_threshold_bytes: usize,

    row_capacity: usize,
    sequence_capacity: usize,
    signal_capacity: usize,
}

const ROW_CAPACITY: usize = 12_000;
const SEQ_CAPACITY: usize = 1000;
const SIG_CAPACITY: usize = 10_000;

impl ParquetBuffer {
    /// Constructs a new `ParquetBuffer` with preallocated capacities.
    ///
    /// # Arguments
    ///
    /// * `row_capacity` - Expected number of records per batch.
    /// * `sequence_capacity` - Expected average length of sequence and mapping arrays.
    /// * `signal_capacity` - Expected average length of signal arrays.
    /// * `flush_threshold_bytes` - Approximate memory threshold for triggering a flush.
    pub fn new(
        flush_threshold_bytes: usize    // Default: 128_000_000
    ) -> Self {
        let read_id = Self::alloc_read_id(ROW_CAPACITY);

        let query_to_sig = Self::alloc_map(
            ROW_CAPACITY,
            SEQ_CAPACITY
        );

        let query_shift = Self::alloc_shift_scale(ROW_CAPACITY);
        let query_scale = Self::alloc_shift_scale(ROW_CAPACITY);

        let ref_to_sig = Self::alloc_map(
            ROW_CAPACITY,
            SEQ_CAPACITY
        );

        let ref_shift = Self::alloc_shift_scale(ROW_CAPACITY);
        let ref_scale = Self::alloc_shift_scale(ROW_CAPACITY);

        let ref_name = Self::alloc_ref_name(ROW_CAPACITY);

        let ref_start = Self::alloc_ref_start(ROW_CAPACITY);

        let query_seq = Self::alloc_seq(
            ROW_CAPACITY,
            SEQ_CAPACITY
        );

        let ref_seq = Self::alloc_seq(
            ROW_CAPACITY,
            SEQ_CAPACITY
        );

        let signal = Self::alloc_signal(
            ROW_CAPACITY,
            SIG_CAPACITY
        );

        Self { 
            read_id,
            query_to_sig, 
            query_shift,
            query_scale,
            ref_to_sig,
            ref_shift,
            ref_scale,
            ref_name,
            ref_start,
            query_seq,
            ref_seq,
            signal,
            current_size_bytes: 0,
            flush_threshold_bytes,
            row_capacity: ROW_CAPACITY,
            sequence_capacity: SEQ_CAPACITY,
            signal_capacity: SIG_CAPACITY
        }
    }

    fn alloc_read_id(row_capacity: usize) -> MutableUtf8Array<i32> {
        MutableUtf8Array::<i32>::with_capacities(
            row_capacity,
            row_capacity * 50
        )
    }

    fn alloc_map(
        row_capacity: usize,
        sequence_capacity: usize
    ) -> MutableListArray<i32, MutablePrimitiveArray<u64>> {
        let mut map = MutableListArray::<i32, MutablePrimitiveArray<u64>>::with_capacity(
            row_capacity
        );
        map
            .mut_values()
            .reserve(row_capacity * sequence_capacity);

        map
    }

    fn alloc_shift_scale(
        row_capacity: usize
    ) -> MutablePrimitiveArray<f32> {
        MutablePrimitiveArray::<f32>::with_capacity(
            row_capacity
        )
    }

    fn alloc_ref_name(
        row_capacity: usize
    ) -> MutableUtf8Array<i32> {
        MutableUtf8Array::<i32>::with_capacities(
            row_capacity,
            row_capacity * 24
        )
    }

    fn alloc_ref_start(
        row_capacity: usize
    ) -> MutablePrimitiveArray<u64> {
        MutablePrimitiveArray::<u64>::with_capacity(
            row_capacity
        )
    }

    fn alloc_seq(
        row_capacity: usize,
        sequence_capacity: usize
    ) -> MutableUtf8Array<i32> {
        MutableUtf8Array::<i32>::with_capacities(
            row_capacity,
            row_capacity * sequence_capacity
        )
    }

    fn alloc_signal(
        row_capacity: usize,
        signal_capacity: usize
    ) -> MutableListArray<i32, MutablePrimitiveArray<f32>> {
        let mut signal = MutableListArray::<i32, MutablePrimitiveArray<f32>>::with_capacity(
            row_capacity
        );
        signal.mut_values().reserve(row_capacity * signal_capacity);
        signal
    }


}

impl<S: OutputSchema> Buffer<S> for ParquetBuffer {
    type FlushOutput = Chunk<Box<dyn Array>>;

    /// Appends a single `OutputRecord` to the buffer.
    ///
    /// Fields are conditionally written based on the compile-time schema `S`.
    /// The internal memory usage estimate is updated accordingly.
    ///
    /// # Errors
    ///
    /// Returns `BufferError` if insertion into any Arrow array fails.
    ///
    /// # Panics
    ///
    /// Panics if a required field (as dictated by `OutputSchema`) is missing
    /// from the provided `OutputRecord`.
    fn push(&mut self, record: OutputRecord) -> Result<(), BufferError> {

        let v= record.read_id;
        self.current_size_bytes += v.len();
        self.read_id.push(Some(v));

        if S::HAS_QUERY_TO_SIGNAL {
            let v = record.query_to_sig.expect("schema guarantees query_to_sig");
            self.current_size_bytes += v.len() * std::mem::size_of::<u64>() + 2 * std::mem::size_of::<f32>();

            self.query_to_sig.try_push(Some(
                v.iter().map(|&el| Some(el as u64))
            ))?;

            let v = record.query_shift.expect("schema guarantees query_shift");
            self.query_shift.try_push(Some(v))?;
            
            let v = record.query_scale.expect("schema guarantees query_scale");
            self.query_scale.try_push(Some(v))?;

        }

        if S::HAS_REF_TO_SIGNAL {
            let v = record.ref_to_sig.expect("schema guarantees ref_to_sig");
            self.current_size_bytes += v.len() * std::mem::size_of::<u64>() + 2 * std::mem::size_of::<f32>();

            self.ref_to_sig.try_push(Some(
                v.iter().map(|&el| Some(el as u64))
            ))?;

            let v = record.ref_shift.expect("schema guarantees ref_shift");
            self.ref_shift.try_push(Some(v))?;
            
            let v = record.ref_scale.expect("schema guarantees ref_scale");
            self.ref_scale.try_push(Some(v))?;
        }

        if S::HAS_REF_META {
            let v = record.ref_name.expect("schema guarantees ref_name");
            self.current_size_bytes += v.len();
            self.ref_name.push(Some(v));

            self.current_size_bytes += std::mem::size_of::<u64>();
            self.ref_start.push(Some(
                record.ref_start.expect("schema guarantees ref_start") as u64
            ));
        }

        if S::HAS_QUERY_SEQ {
            let v: &str = &record.query_seq.expect("schema guarantees query_seq");
            self.current_size_bytes += v.len();
            self.query_seq.push(Some(v));
        }

        if S::HAS_REF_SEQ {
            let v: &str = &record.ref_seq.expect("schema guarantees ref_seq");
            self.current_size_bytes += v.len();
            self.ref_seq.push(Some(v));
        }

        if S::HAS_SIGNAL {
            let v = record.signal.expect("schema guarantees signal");
            self.current_size_bytes += v.len() * std::mem::size_of::<f32>();
            self.signal.try_push(Some(
                v.iter().map(|&el| Some(el))
            ))?;
        }

        Ok(())
    }

    /// Returns `true` if the estimated memory usage exceeds the configured
    /// flush threshold.
    ///
    /// This is based on an approximate byte count accumulated during `push`.
    fn should_flush(&self) -> bool {
        self.current_size_bytes >= self.flush_threshold_bytes
    }

    /// Converts the buffered data into an Arrow `Chunk` and resets the buffer.
    ///
    /// This operation:
    /// - Transfers ownership of all internal arrays into a `Chunk`
    /// - Reinitializes buffers with preallocated capacity
    /// - Resets the internal memory usage counter
    ///
    /// The resulting `Chunk` is suitable for direct consumption by a Parquet writer.
    ///
    /// # Guarantees
    ///
    /// - Column order and presence strictly follow `OutputSchema`.
    /// - All columns have equal length.
    ///
    /// # Errors
    ///
    /// Returns `BufferError` if array conversion fails.
    fn flush(&mut self) -> Result<Chunk<Box<dyn Array>>, BufferError> {
        let mut columns: Vec<Box<dyn Array>> = vec![];

        let mut read_id = replace(
            &mut self.read_id,
            Self::alloc_read_id(self.row_capacity)
        );
        columns.push(read_id.as_box());

        if S::HAS_QUERY_TO_SIGNAL {
            let mut query_to_sig = replace(
                &mut self.query_to_sig,
                Self::alloc_map(self.row_capacity, self.sequence_capacity)
            );
            columns.push(query_to_sig.as_box());

            let mut query_shift = replace(
                &mut self.query_shift,
                Self::alloc_shift_scale(self.row_capacity)
            );
            columns.push(query_shift.as_box());

            let mut query_scale = replace(
                &mut self.query_scale,
                Self::alloc_shift_scale(self.row_capacity)
            );
            columns.push(query_scale.as_box());
        }

        if S::HAS_REF_TO_SIGNAL {
            let mut ref_to_sig = replace(
                &mut self.ref_to_sig,
                Self::alloc_map(self.row_capacity, self.sequence_capacity)
            );
            columns.push(ref_to_sig.as_box());

            let mut ref_shift = replace(
                &mut self.ref_shift,
                Self::alloc_shift_scale(self.row_capacity)
            );
            columns.push(ref_shift.as_box());

            let mut ref_scale = replace(
                &mut self.ref_scale,
                Self::alloc_shift_scale(self.row_capacity)
            );
            columns.push(ref_scale.as_box());
        }

        if S::HAS_REF_META {
            let mut ref_name = replace(
                &mut self.ref_name,
                Self::alloc_ref_name(self.row_capacity)
            );
            columns.push(ref_name.as_box());

            let mut ref_start = replace(
                &mut self.ref_start,
                Self::alloc_ref_start(self.row_capacity)
            );
            columns.push(ref_start.as_box());
        }

        if S::HAS_QUERY_SEQ {
            let mut query_seq = replace(
                &mut self.query_seq,
                Self::alloc_seq(self.row_capacity, self.sequence_capacity)  
            );
            columns.push(query_seq.as_box());
        }

        if S::HAS_REF_SEQ {
            let mut ref_seq = replace(
                &mut self.ref_seq,
                Self::alloc_seq(self.row_capacity, self.sequence_capacity)  
            );
            columns.push(ref_seq.as_box());
        }

        if S::HAS_SIGNAL {
            let mut signal = replace(
                &mut self.signal,
                Self::alloc_signal(self.row_capacity, self.signal_capacity)
            );
            columns.push(signal.as_box());
        }

        self.current_size_bytes = 0;

        Ok(Chunk::new(columns))
    }
}