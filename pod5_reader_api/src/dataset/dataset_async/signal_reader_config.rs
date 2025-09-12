use std:: path::PathBuf;

/// Configuration for initializing a FeatherReader for signal table access in a Pod5 file.
/// 
/// This struct encapsulates all the metadata required to create and configure a
/// `FeatherReader` for accessing the signal table within a specific Pod5 file.
/// It's used by the reader pool to efficiently initialize readers on-demand.
/// 
/// # Fields
/// 
/// * `file_id` - Unique identifier for the file within the dataset (0-based index)
/// * `path` - Path to the Pod5 file
/// * `offset` - Byte offset where the signal table begins within the file
/// * `length` - Size of the signal table in bytes
/// * `batch_size` - Number of rows to process per batch when reading the signal table
#[derive(Debug, Clone)]
pub(super) struct SignalReaderConfig {
    pub(super) file_id: usize,
    pub(super) path: PathBuf,
    pub(super) offset: i64,
    pub(super) length: i64,
    pub(super) batch_size: u64
} 

impl SignalReaderConfig {
    /// Creates a new signal reader configuration.
    /// 
    /// # Arguments
    /// 
    /// * `file_id` - Index of the file within the dataset
    /// * `path` - Path to the Pod5 file
    /// * `offset` - Byte offset of the signal table within the file
    /// * `length` - Length of the signal table in bytes
    /// * `batch_size` - Number of rows per batch for signal table processing
    /// 
    /// # Returns
    /// 
    /// A new `SignalReaderConfig` instance with the specified parameters.
    pub(super) fn new(
        file_id: usize, 
        path: &PathBuf, 
        offset: i64, 
        length: i64,
        batch_size: u64
    ) -> Self {
        Self { 
            file_id: file_id.clone(), 
            path: path.clone(), 
            offset, 
            length, 
            batch_size
        }
    }
}
