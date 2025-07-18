use std::sync::Arc;

use arrow::{
    array::{
        ArrayRef, 
        Int16Builder, 
        ListBuilder, 
        RecordBatch, 
        StringBuilder, 
        UInt64Builder
    }, 
    datatypes::Schema
};
use crate::{
    cli::{
        output::{
            output_data::OutputData, 
            OutputConfig
        }, 
        parse::args_to_input::WhichToAlign
    }, 
    error::output_errors::OutputError
};

/// A builder for creating Arrow RecordBatches from alignment data.
/// 
/// Handles different combinations of alignment outputs with optional fields like sequences and signal data.
/// Contains variants for all combinations of alignment types (Query/Reference/Both) 
/// and optional fields (sequences/signal).
#[derive(Debug)]
pub enum ArrowBuffer {
    QueryBasic{
        buffer_read_id: StringBuilder,
        buffer_query_to_signal: ListBuilder<UInt64Builder>,
    },
    RefBasic{
        buffer_read_id: StringBuilder,
        buffer_ref_to_signal: ListBuilder<UInt64Builder>,
        buffer_ref_name: StringBuilder,
        buffer_ref_start: UInt64Builder,
    },
    BothBasic{
        buffer_read_id: StringBuilder,
        buffer_query_to_signal: ListBuilder<UInt64Builder>,
        buffer_ref_to_signal: ListBuilder<UInt64Builder>,
        buffer_ref_name: StringBuilder,
        buffer_ref_start: UInt64Builder,
    },
    QueryWithSeq{
        buffer_read_id: StringBuilder,
        buffer_query_to_signal: ListBuilder<UInt64Builder>,
        buffer_query_seq: StringBuilder,
    },
    RefWithSeq{
        buffer_read_id: StringBuilder,
        buffer_ref_to_signal: ListBuilder<UInt64Builder>,
        buffer_ref_name: StringBuilder,
        buffer_ref_start: UInt64Builder,
        buffer_ref_seq: StringBuilder,
    },
    BothWithSeq{
        buffer_read_id: StringBuilder,
        buffer_query_to_signal: ListBuilder<UInt64Builder>,
        buffer_ref_to_signal: ListBuilder<UInt64Builder>,
        buffer_ref_name: StringBuilder,
        buffer_ref_start: UInt64Builder,
        buffer_query_seq: StringBuilder,
        buffer_ref_seq: StringBuilder,
    },
    QueryWithSeqAndSig{
        buffer_read_id: StringBuilder,
        buffer_query_to_signal: ListBuilder<UInt64Builder>,
        buffer_query_seq: StringBuilder,
        buffer_signal: ListBuilder<Int16Builder>,
    },
    RefWithSeqAndSig{
        buffer_read_id: StringBuilder,
        buffer_ref_to_signal: ListBuilder<UInt64Builder>,
        buffer_ref_name: StringBuilder,
        buffer_ref_start: UInt64Builder,
        buffer_ref_seq: StringBuilder,
        buffer_signal: ListBuilder<Int16Builder>,
    },
    BothWithSeqAndSig{
        buffer_read_id: StringBuilder,
        buffer_query_to_signal: ListBuilder<UInt64Builder>,
        buffer_ref_to_signal: ListBuilder<UInt64Builder>,
        buffer_ref_name: StringBuilder,
        buffer_ref_start: UInt64Builder,
        buffer_query_seq: StringBuilder,
        buffer_ref_seq: StringBuilder,
        buffer_signal: ListBuilder<Int16Builder>,
    }
}

impl ArrowBuffer {
    /// Creates a new StringBuilder for string fields (for read_id, sequences)
    fn new_string_builder() -> StringBuilder {
        StringBuilder::new()
    }

    /// Creates a new ListBuilder for UInt64 lists (for the alignments)
    fn new_u64_list_builder() -> ListBuilder<UInt64Builder> {
        ListBuilder::<UInt64Builder>::new(UInt64Builder::new())
    }

    /// Creates a new builder for UInt64 fields (for ref_start)
    fn new_u64_builder() -> UInt64Builder {
        UInt64Builder::new()
    }

    /// Creates a new ListBuilder for Int16 lists (for the signal data)
    fn new_i16_list_builder() -> ListBuilder<Int16Builder> {
        ListBuilder::<Int16Builder>::new(Int16Builder::new())
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

    /// Appends a string value to a StringBuilder
    fn append_string(builder: &mut StringBuilder, value: &str) {
        builder.append_value(value);
    }

    /// Appends an optional string value to a StringBuilder
    fn append_optional_string(builder: &mut StringBuilder, value: &Option<String>) {
        if let Some(s) = value {
            builder.append_value(s);
        } else {
            builder.append_null();
        }
    }

    /// Appends an optional usize list to a UInt64 ListBuilder
    fn append_optional_usize_list(builder: &mut ListBuilder<UInt64Builder>, value: &Option<Vec<usize>>) {
        if let Some(vec) = value {
            builder.append_value(vec.iter().map(|&x| Some(x as u64)));
        } else {
            builder.append_null();
        }
    }

    /// Appends an optional usize value to a UInt64 builder
    fn append_optional_usize(builder: &mut UInt64Builder, value: &Option<usize>) {
        if let Some(val) = value {
            builder.append_value(*val as u64);
        } else {
            builder.append_null();
        }
    }

    /// Appends an optional i16 list to an Int16 ListBuilder (for signal data)
    fn append_optional_i16_list(builder: &mut ListBuilder<Int16Builder>, value: &Option<Vec<i16>>) {
        if let Some(vec) = value {
            builder.append_value(vec.iter().map(|&x| Some(x)));
        } else {
            builder.append_null();
        }
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
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal);
            }

            (ArrowBuffer::RefBasic { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start }, 
             OutputData::RefBasic { read_id, ref_to_signal, ref_name, ref_start }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
            }

            (ArrowBuffer::BothBasic { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start }, 
             OutputData::BothBasic { read_id, query_to_signal, ref_to_signal, ref_name, ref_start }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
            }

            (ArrowBuffer::QueryWithSeq { buffer_read_id, buffer_query_to_signal, buffer_query_seq }, 
             OutputData::QueryWithSeq { read_id, query_to_signal, query_sequence }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal);
                Self::append_optional_string(buffer_query_seq, query_sequence);
            }

            (ArrowBuffer::RefWithSeq { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_ref_seq }, 
             OutputData::RefWithSeq { read_id, ref_to_signal, ref_sequence, ref_name, ref_start }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal);
                Self::append_optional_string(buffer_ref_seq, ref_sequence);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
            }

            (ArrowBuffer::BothWithSeq { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_query_seq, buffer_ref_seq }, 
             OutputData::BothWithSeq { read_id, query_to_signal, query_sequence, ref_to_signal, ref_sequence, ref_name, ref_start }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal);
                Self::append_optional_string(buffer_query_seq, query_sequence);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal);
                Self::append_optional_string(buffer_ref_seq, ref_sequence);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
            }

            (ArrowBuffer::QueryWithSeqAndSig { buffer_read_id, buffer_query_to_signal, buffer_query_seq, buffer_signal }, 
             OutputData::QueryWithSeqAndSig { read_id, query_to_signal, query_sequence, signal }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal);
                Self::append_optional_string(buffer_query_seq, query_sequence);
                Self::append_optional_i16_list(buffer_signal, signal);
            }

            (ArrowBuffer::RefWithSeqAndSig { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_ref_seq, buffer_signal }, 
             OutputData::RefWithSeqAndSig { read_id, ref_to_signal, ref_sequence, ref_name, ref_start, signal }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal);
                Self::append_optional_string(buffer_ref_seq, ref_sequence);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
                Self::append_optional_i16_list(buffer_signal, signal);
            }

            (ArrowBuffer::BothWithSeqAndSig { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_query_seq, buffer_ref_seq, buffer_signal }, 
             OutputData::BothWithSeqAndSig { read_id, query_to_signal, query_sequence, ref_to_signal, ref_sequence, ref_name, ref_start, signal }) => {
                Self::append_string(buffer_read_id, read_id);
                Self::append_optional_usize_list(buffer_query_to_signal, query_to_signal);
                Self::append_optional_string(buffer_query_seq, query_sequence);
                Self::append_optional_usize_list(buffer_ref_to_signal, ref_to_signal);
                Self::append_optional_string(buffer_ref_seq, ref_sequence);
                Self::append_optional_string(buffer_ref_name, ref_name);
                Self::append_optional_usize(buffer_ref_start, ref_start);
                Self::append_optional_i16_list(buffer_signal, signal);
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
    pub fn buffer_to_record_batch(&mut self, schema: &Arc<Schema>) -> Result<RecordBatch, OutputError> {
        let mut columns: Vec<ArrayRef> = vec![];

        match self {
            ArrowBuffer::QueryBasic { buffer_read_id, buffer_query_to_signal } => {
                columns.push(Arc::new(buffer_read_id.finish()));
                columns.push(Arc::new(buffer_query_to_signal.finish()));
            }
            ArrowBuffer::RefBasic { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start } => {
                columns.push(Arc::new(buffer_read_id.finish()));
                columns.push(Arc::new(buffer_ref_to_signal.finish()));
                columns.push(Arc::new(buffer_ref_name.finish()));
                columns.push(Arc::new(buffer_ref_start.finish()));
            }
            ArrowBuffer::BothBasic { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start } => {
                columns.push(Arc::new(buffer_read_id.finish()));
                columns.push(Arc::new(buffer_query_to_signal.finish()));
                columns.push(Arc::new(buffer_ref_to_signal.finish()));
                columns.push(Arc::new(buffer_ref_name.finish()));
                columns.push(Arc::new(buffer_ref_start.finish()));
            }
            ArrowBuffer::QueryWithSeq { buffer_read_id, buffer_query_to_signal, buffer_query_seq } => {
                columns.push(Arc::new(buffer_read_id.finish()));
                columns.push(Arc::new(buffer_query_to_signal.finish()));
                columns.push(Arc::new(buffer_query_seq.finish()));
            }
            ArrowBuffer::RefWithSeq { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_ref_seq } => {
                columns.push(Arc::new(buffer_read_id.finish()));
                columns.push(Arc::new(buffer_ref_to_signal.finish()));
                columns.push(Arc::new(buffer_ref_name.finish()));
                columns.push(Arc::new(buffer_ref_start.finish()));
                columns.push(Arc::new(buffer_ref_seq.finish()));
            }
            ArrowBuffer::BothWithSeq { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_query_seq, buffer_ref_seq } => {
                columns.push(Arc::new(buffer_read_id.finish()));
                columns.push(Arc::new(buffer_query_to_signal.finish()));
                columns.push(Arc::new(buffer_ref_to_signal.finish()));
                columns.push(Arc::new(buffer_ref_name.finish()));
                columns.push(Arc::new(buffer_ref_start.finish()));
                columns.push(Arc::new(buffer_query_seq.finish()));
                columns.push(Arc::new(buffer_ref_seq.finish()));
            }
            ArrowBuffer::QueryWithSeqAndSig { buffer_read_id, buffer_query_to_signal, buffer_query_seq, buffer_signal } => {
                columns.push(Arc::new(buffer_read_id.finish()));
                columns.push(Arc::new(buffer_query_to_signal.finish()));
                columns.push(Arc::new(buffer_query_seq.finish()));
                columns.push(Arc::new(buffer_signal.finish()));
            }
            ArrowBuffer::RefWithSeqAndSig { buffer_read_id, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_ref_seq, buffer_signal } => {
                columns.push(Arc::new(buffer_read_id.finish()));
                columns.push(Arc::new(buffer_ref_to_signal.finish()));
                columns.push(Arc::new(buffer_ref_name.finish()));
                columns.push(Arc::new(buffer_ref_start.finish()));
                columns.push(Arc::new(buffer_ref_seq.finish()));
                columns.push(Arc::new(buffer_signal.finish()));
            }
            ArrowBuffer::BothWithSeqAndSig { buffer_read_id, buffer_query_to_signal, buffer_ref_to_signal, buffer_ref_name, buffer_ref_start, buffer_query_seq, buffer_ref_seq, buffer_signal } => {
                columns.push(Arc::new(buffer_read_id.finish()));
                columns.push(Arc::new(buffer_query_to_signal.finish()));
                columns.push(Arc::new(buffer_ref_to_signal.finish()));
                columns.push(Arc::new(buffer_ref_name.finish()));
                columns.push(Arc::new(buffer_ref_start.finish()));
                columns.push(Arc::new(buffer_query_seq.finish()));
                columns.push(Arc::new(buffer_ref_seq.finish()));
                columns.push(Arc::new(buffer_signal.finish()));
            }
        }

        RecordBatch::try_new(schema.clone(), columns)
            .map_err(|e| OutputError::ArrowError(e))
    }
}
