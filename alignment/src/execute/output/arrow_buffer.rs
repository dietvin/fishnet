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
    datatypes::Schema, 
    io::parquet::write::{
        Encoding,
        RowGroupIterator,
        WriteOptions
    },
};
use crate::{
    error::output_errors::OutputError, execute::{config::{output_config::OutputConfig, WhichToAlign}, output::output_data::OutputData}
};

/// A builder for creating Arrow RecordBatches from alignment data.
/// 
/// Handles different combinations of alignment outputs with optional fields like sequences and signal data.
/// Contains variants for all combinations of alignment types (Query/Reference/Both) 
/// and optional fields (sequences/signal).
#[derive(Debug)]
pub enum ArrowBuffer {
    QueryBasic{
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_query_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
    },
    RefBasic{
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_ref_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_ref_name: MutableUtf8Array<i32>,
        buffer_ref_start: MutablePrimitiveArray<u64>,
    },
    BothBasic{
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_query_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_ref_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_ref_name: MutableUtf8Array<i32>,
        buffer_ref_start: MutablePrimitiveArray<u64>,
    },
    QueryWithSeq{
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_query_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_query_seq: MutableUtf8Array<i32>,
    },
    RefWithSeq{
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_ref_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_ref_name: MutableUtf8Array<i32>,
        buffer_ref_start: MutablePrimitiveArray<u64>,
        buffer_ref_seq: MutableUtf8Array<i32>,
    },
    BothWithSeq{
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_query_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_ref_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_ref_name: MutableUtf8Array<i32>,
        buffer_ref_start: MutablePrimitiveArray<u64>,
        buffer_query_seq: MutableUtf8Array<i32>,
        buffer_ref_seq: MutableUtf8Array<i32>,
    },
    QueryWithSeqAndSig{
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_query_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_query_seq: MutableUtf8Array<i32>,
        buffer_signal: MutableListArray<i32, MutablePrimitiveArray<i16>>,
    },
    RefWithSeqAndSig{
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_ref_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_ref_name: MutableUtf8Array<i32>,
        buffer_ref_start: MutablePrimitiveArray<u64>,
        buffer_ref_seq: MutableUtf8Array<i32>,
        buffer_signal: MutableListArray<i32, MutablePrimitiveArray<i16>>,
    },
    BothWithSeqAndSig{
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_query_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_ref_to_signal: MutableListArray<i32, MutablePrimitiveArray<u64>>,
        buffer_ref_name: MutableUtf8Array<i32>,
        buffer_ref_start: MutablePrimitiveArray<u64>,
        buffer_query_seq: MutableUtf8Array<i32>,
        buffer_ref_seq: MutableUtf8Array<i32>,
        buffer_signal: MutableListArray<i32, MutablePrimitiveArray<i16>>,
    }
}

impl ArrowBuffer {
    /// Creates a new MutableUtf8Array<i32> for string fields (for read_id, sequences)
    fn new_string_builder() -> MutableUtf8Array<i32> {
        MutableUtf8Array::<i32>::new()
    }

    /// Creates a new ListBuilder for UInt64 lists (for the alignments)
    fn new_u64_list_builder() -> MutableListArray<i32, MutablePrimitiveArray<u64>> {
        MutableListArray::<i32, MutablePrimitiveArray<u64>>::new()
    }

    /// Creates a new builder for UInt64 fields (for ref_start)
    fn new_u64_builder() -> MutablePrimitiveArray<u64> {
        MutablePrimitiveArray::<u64>::new()
    }

    /// Creates a new ListBuilder for Int16 lists (for the signal data)
    fn new_i16_list_builder() -> MutableListArray<i32, MutablePrimitiveArray<i16>> {
        MutableListArray::<i32, MutablePrimitiveArray<i16>>::new()
    }

    /// Creates a new ArrowBuffer based on output configuration
    ///
    /// # Arguments
    /// * `output_config` - Configuration specifying which fields to include
    ///
    /// # Returns
    /// Appropriate ArrowBuffer variant based on configuration
    pub fn new(output_config: &OutputConfig) -> Self {
        let alignment_type = output_config.alignment_type();
        let include_sequences = output_config.include_sequences();
        let include_signal = output_config.include_signal();
        match (alignment_type, include_sequences, include_signal) {
            (WhichToAlign::Query, false, false) => ArrowBuffer::QueryBasic {
                buffer_read_id: Self::new_string_builder(),
                buffer_query_to_signal: Self::new_u64_list_builder(),
            },
            (WhichToAlign::Reference, false, false) => ArrowBuffer::RefBasic {
                buffer_read_id: Self::new_string_builder(),
                buffer_ref_to_signal: Self::new_u64_list_builder(),
                buffer_ref_name: Self::new_string_builder(),
                buffer_ref_start: Self::new_u64_builder(),
            },
            (WhichToAlign::Both, false, false) => ArrowBuffer::BothBasic {
                buffer_read_id: Self::new_string_builder(),
                buffer_query_to_signal: Self::new_u64_list_builder(),
                buffer_ref_to_signal: Self::new_u64_list_builder(),
                buffer_ref_name: Self::new_string_builder(),
                buffer_ref_start: Self::new_u64_builder(),
            },
            (WhichToAlign::Query, true, false) => ArrowBuffer::QueryWithSeq {
                buffer_read_id: Self::new_string_builder(),
                buffer_query_to_signal: Self::new_u64_list_builder(),
                buffer_query_seq: Self::new_string_builder(),
            },
            (WhichToAlign::Reference, true, false) => ArrowBuffer::RefWithSeq {
                buffer_read_id: Self::new_string_builder(),
                buffer_ref_to_signal: Self::new_u64_list_builder(),
                buffer_ref_name: Self::new_string_builder(),
                buffer_ref_start: Self::new_u64_builder(),
                buffer_ref_seq: Self::new_string_builder(),
            },
            (WhichToAlign::Both, true, false) => ArrowBuffer::BothWithSeq {
                buffer_read_id: Self::new_string_builder(),
                buffer_query_to_signal: Self::new_u64_list_builder(),
                buffer_ref_to_signal: Self::new_u64_list_builder(),
                buffer_ref_name: Self::new_string_builder(),
                buffer_ref_start: Self::new_u64_builder(),
                buffer_query_seq: Self::new_string_builder(),
                buffer_ref_seq: Self::new_string_builder(),
            },
            (WhichToAlign::Query, true, true) => ArrowBuffer::QueryWithSeqAndSig {
                buffer_read_id: Self::new_string_builder(),
                buffer_query_to_signal: Self::new_u64_list_builder(),
                buffer_query_seq: Self::new_string_builder(),
                buffer_signal: Self::new_i16_list_builder(),
            },
            (WhichToAlign::Reference, true, true) => ArrowBuffer::RefWithSeqAndSig {
                buffer_read_id: Self::new_string_builder(),
                buffer_ref_to_signal: Self::new_u64_list_builder(),
                buffer_ref_name: Self::new_string_builder(),
                buffer_ref_start: Self::new_u64_builder(),
                buffer_ref_seq: Self::new_string_builder(),
                buffer_signal: Self::new_i16_list_builder(),
            },
            (WhichToAlign::Both, true, true) => ArrowBuffer::BothWithSeqAndSig {
                buffer_read_id: Self::new_string_builder(),
                buffer_query_to_signal: Self::new_u64_list_builder(),
                buffer_ref_to_signal: Self::new_u64_list_builder(),
                buffer_ref_name: Self::new_string_builder(),
                buffer_ref_start: Self::new_u64_builder(),
                buffer_query_seq: Self::new_string_builder(),
                buffer_ref_seq: Self::new_string_builder(),
                buffer_signal: Self::new_i16_list_builder(),
            },
            _ => unreachable!()
        }
    }

    /// Appends a string value to a MutableUtf8Array<i32>
    fn append_string(builder: &mut MutableUtf8Array<i32>, value: &str) {
        builder.push(Some(value));
    }

    /// Appends an optional string value to a MutableUtf8Array<i32>
    fn append_optional_string(builder: &mut MutableUtf8Array<i32>, value: &Option<String>) {
        builder.push(value.as_deref());
    }

    /// Appends an optional usize list to a UInt64 ListBuilder
    fn append_optional_usize_list(builder: &mut MutableListArray<i32, MutablePrimitiveArray<u64>>, value: &Option<Vec<usize>>) -> Result<(), arrow2::error::Error> {
        match value {
            Some(vec) => {
                let vec_u64 = vec.iter().map(|&x| Some(x as u64)).collect::<Vec<Option<u64>>>();
                builder.try_push(Some(vec_u64))?;
            }
            None => {
                let opt: Option<Vec<Option<u64>>> = None;
                builder.try_push(opt)?;
            }
        }
        Ok(())
    }

    /// Appends an optional usize value to a UInt64 builder
    fn append_optional_usize(builder: &mut MutablePrimitiveArray<u64>, value: &Option<usize>) {
        builder.push(value.map(|v| v as u64));
    }

    /// Appends an optional i16 list to an Int16 ListBuilder (for signal data)
    fn append_optional_i16_list(
        builder: &mut MutableListArray<i32, MutablePrimitiveArray<i16>>, 
        value: &Option<Vec<i16>>
    ) -> Result<(), arrow2::error::Error> {
        match value {
            Some(vec) => {
                let vec_i16 = vec.iter().map(|&x| Some(x as i16)).collect::<Vec<Option<i16>>>();
                builder.try_push(Some(vec_i16))?;
            }
            None => {
                let opt: Option<Vec<Option<i16>>> = None;
                builder.try_push(opt)?;
            }
        }
        Ok(())
    }

    pub fn variant_name(&self) -> &'static str {
        match self {
            ArrowBuffer::QueryBasic { .. } => "QueryBasic",
            ArrowBuffer::RefBasic { .. } => "RefBasic",
            ArrowBuffer::BothBasic { .. } => "BothBasic",
            ArrowBuffer::QueryWithSeq { .. } => "QueryWithSeq",
            ArrowBuffer::RefWithSeq { .. } => "RefWithSeq",
            ArrowBuffer::BothWithSeq { .. } => "BothWithSeq",
            ArrowBuffer::QueryWithSeqAndSig { .. } => "QueryWithSeqAndSig",
            ArrowBuffer::RefWithSeqAndSig { .. } => "RefWithSeqAndSig",
            ArrowBuffer::BothWithSeqAndSig { .. } => "BothWithSeqAndSig",
        }
    }

    /// Appends data to the buffer
    ///
    /// # Arguments
    /// * `data` - The output data to append
    ///
    /// # Returns
    /// Result indicating success or failure (if variant mismatch occurs)
    ///
    /// # Errors
    /// Returns OutputError if data variant doesn't match buffer variant
    pub fn push_data(&mut self, data: &OutputData) -> Result<(), OutputError> {
        let buffer_variant_name = self.variant_name().to_string();
        match (self, data) {
            (ArrowBuffer::QueryBasic { buffer_read_id, buffer_query_to_signal }, 
             OutputData::QueryBasic { read_id, query_to_signal }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal)?;
            }

            (ArrowBuffer::RefBasic { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start }, 
             OutputData::RefBasic { read_id, ref_to_signal, ref_name, ref_start }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal)?;
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
            }

            (ArrowBuffer::BothBasic { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start }, 
             OutputData::BothBasic { read_id, query_to_signal, ref_to_signal, ref_name, ref_start }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal)?;
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal)?;
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
            }

            (ArrowBuffer::QueryWithSeq { buffer_read_id, buffer_query_to_signal, buffer_query_seq }, 
             OutputData::QueryWithSeq { read_id, query_to_signal, query_sequence }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal)?;
                Self::append_optional_string(buffer_query_seq, query_sequence);
            }

            (ArrowBuffer::RefWithSeq { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_ref_seq }, 
             OutputData::RefWithSeq { read_id, ref_to_signal, ref_sequence, ref_name, ref_start }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal)?;
                Self::append_optional_string(buffer_ref_seq, ref_sequence);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
            }

            (ArrowBuffer::BothWithSeq { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_query_seq, buffer_ref_seq }, 
             OutputData::BothWithSeq { read_id, query_to_signal, query_sequence, ref_to_signal, ref_sequence, ref_name, ref_start }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal)?;
                Self::append_optional_string(buffer_query_seq, query_sequence);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal)?;
                Self::append_optional_string(buffer_ref_seq, ref_sequence);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
            }

            (ArrowBuffer::QueryWithSeqAndSig { buffer_read_id, buffer_query_to_signal, buffer_query_seq, buffer_signal }, 
             OutputData::QueryWithSeqAndSig { read_id, query_to_signal, query_sequence, signal }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal)?;
                Self::append_optional_string(buffer_query_seq, query_sequence);
                Self::append_optional_i16_list(buffer_signal, signal)?;
            }

            (ArrowBuffer::RefWithSeqAndSig { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_ref_seq, buffer_signal }, 
             OutputData::RefWithSeqAndSig { read_id, ref_to_signal, ref_sequence, ref_name, ref_start, signal }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal)?;
                Self::append_optional_string(buffer_ref_seq, ref_sequence);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
                Self::append_optional_i16_list(buffer_signal, signal)?;
            }

            (ArrowBuffer::BothWithSeqAndSig { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_query_seq, buffer_ref_seq, buffer_signal }, 
             OutputData::BothWithSeqAndSig { read_id, query_to_signal, query_sequence, ref_to_signal, ref_sequence, ref_name, ref_start, signal }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal)?;
                Self::append_optional_string(buffer_query_seq, query_sequence);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal)?;
                Self::append_optional_string(buffer_ref_seq, ref_sequence);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
                Self::append_optional_i16_list(buffer_signal, signal)?;
            }

            // Mismatched variants return an error
            _ => return Err(OutputError::MismatchedVariants(
                buffer_variant_name,
                data.variant_name().to_string()
            )),
        }

        Ok(())
    }

    /// Converts the buffered data into an Arrow RecordBatch
    ///
    /// # Arguments
    /// * `schema` - The schema to use for the RecordBatch
    ///
    /// # Returns
    /// Result containing the RecordBatch or an error
    ///
    /// # Errors
    /// Returns OutputError if RecordBatch creation fails
    pub fn buffer_to_rowgroupiter(&mut self, schema: &Schema, encodings: &Vec<Vec<Encoding>>, options: &WriteOptions) -> Result<RowGroupIterator<Box<dyn Array>, std::iter::Once<Result<Chunk<Box<dyn Array>>, arrow2::error::Error>>>, OutputError> {
        let mut columns: Vec<Box<dyn Array>> = vec![];

        match self {
            ArrowBuffer::QueryBasic { buffer_read_id, buffer_query_to_signal } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_query_to_signal.as_box());
            }
            ArrowBuffer::RefBasic { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_ref_to_signal.as_box());
                columns.push(buffer_ref_name.as_box());
                columns.push(buffer_ref_start.as_box());
            }
            ArrowBuffer::BothBasic { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_query_to_signal.as_box());
                columns.push(buffer_ref_to_signal.as_box());
                columns.push(buffer_ref_name.as_box());
                columns.push(buffer_ref_start.as_box());
            }
            ArrowBuffer::QueryWithSeq { buffer_read_id, buffer_query_to_signal, buffer_query_seq } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_query_to_signal.as_box());
                columns.push(buffer_query_seq.as_box());
            }
            ArrowBuffer::RefWithSeq { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_ref_seq } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_ref_to_signal.as_box());
                columns.push(buffer_ref_name.as_box());
                columns.push(buffer_ref_start.as_box());
                columns.push(buffer_ref_seq.as_box());
            }
            ArrowBuffer::BothWithSeq { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_query_seq, buffer_ref_seq } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_query_to_signal.as_box());
                columns.push(buffer_ref_to_signal.as_box());
                columns.push(buffer_ref_name.as_box());
                columns.push(buffer_ref_start.as_box());
                columns.push(buffer_query_seq.as_box());
                columns.push(buffer_ref_seq.as_box());
            }
            ArrowBuffer::QueryWithSeqAndSig { buffer_read_id, buffer_query_to_signal, buffer_query_seq, buffer_signal } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_query_to_signal.as_box());
                columns.push(buffer_query_seq.as_box());
                columns.push(buffer_signal.as_box());
            }
            ArrowBuffer::RefWithSeqAndSig { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_ref_seq, buffer_signal } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_ref_to_signal.as_box());
                columns.push(buffer_ref_name.as_box());
                columns.push(buffer_ref_start.as_box());
                columns.push(buffer_ref_seq.as_box());
                columns.push(buffer_signal.as_box());
            }
            ArrowBuffer::BothWithSeqAndSig { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_query_seq, buffer_ref_seq, buffer_signal } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_query_to_signal.as_box());
                columns.push(buffer_ref_to_signal.as_box());
                columns.push(buffer_ref_name.as_box());
                columns.push(buffer_ref_start.as_box());
                columns.push(buffer_query_seq.as_box());
                columns.push(buffer_ref_seq.as_box());
                columns.push(buffer_signal.as_box());
            }
        }

        let chunk = Chunk::try_new(columns)
            .map_err(|e| OutputError::ArrowError(e)).unwrap();

        Ok(RowGroupIterator::try_new(
            std::iter::once(Ok(chunk)), 
            schema, 
            options.clone(), 
            encodings.clone()
        )?)
    }
}
