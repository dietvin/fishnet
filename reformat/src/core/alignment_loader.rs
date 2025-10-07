/// Parquet/JSON input wrapper for genomic alignment data processing.
/// 
/// This module provides row-wise data reading with lazy loading of metadata.
/// It supports both embedded signal data and external Pod5 dataset integration.

pub(crate) mod row;
mod alignment_chunk;
mod column_index;

// Modules for single-threaded iteration
pub(crate) mod row_iterator;

// Modules for multi-threaded iteration
pub(crate) mod raw_row_iterator;
pub(crate) mod raw_row_data;

mod stats;