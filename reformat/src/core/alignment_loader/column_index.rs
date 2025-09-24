use std::collections::HashMap;
use arrow2::datatypes::Schema;

use crate::{
    error::core::loader::ColumnIndexError, 
    execute::config::Column
};


/// Maps column names to their indices in the parquet schema.
/// 
/// This struct provides efficient access to column data by maintaining
/// the mapping between the column types and their positions in the Arrow
/// schema.
pub(super) struct ColumnIndex {
    /// Index of the read_id column (always required)
    pub(super) read_id: usize,
    /// Index of the alignment column (query_to_signal or ref_to_signal)
    pub(super) alignment: usize,
    /// Index of the sequence column (query_sequence or ref_sequence), if present
    pub(super) sequence: Option<usize>,
    /// Index of the reference name column, if present
    pub(super) ref_name: Option<usize>,
    /// Index of the reference start position column, if present
    pub(super) ref_start: Option<usize>,
    /// Index of the signal data column, if embedded in parquet
    pub(super) signal: Option<usize>
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
    pub(super) fn from_schema(
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
