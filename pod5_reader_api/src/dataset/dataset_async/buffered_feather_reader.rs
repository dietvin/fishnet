use std::fs::File;
use crate::{
    dataset::dataset_async::signal_reader_config::SignalReaderConfig, 
    error::dataset::BufferedFeatherReaderError, 
    core::feather_reader::FeatherReader
};


/// A buffered FeatherReader with associated file metadata.
/// 
/// This wrapper combines a `FeatherReader` with its corresponding file identifier,
/// enabling the reader pool to efficiently manage and identify readers for different files.
/// The reader maintains the file context necessary for proper reader reuse and validation.
pub(super) struct BufferedFeatherReader {
    /// The underlying FeatherReader for file access
    pub(super) reader: FeatherReader,
    /// File identifier matching the dataset's file indexing
    pub(super) file_id: usize
}

impl BufferedFeatherReader {
    /// Creates a new BufferedFeatherReader from a configuration.
    /// 
    /// This method opens the specified file and initializes a FeatherReader
    /// configured for the signal table access parameters defined in the config.
    /// 
    /// # Arguments
    /// 
    /// * `config` - Signal reader configuration specifying file location and parameters
    /// 
    /// # Returns
    /// 
    /// A new `BufferedFeatherReader` ready for signal table access, or an error if
    /// file opening or reader initialization fails.
    /// 
    /// # Errors
    /// 
    /// * File system errors when opening the Pod5 file
    /// * FeatherReader initialization errors for malformed files
    pub(super) fn from_config(config: &SignalReaderConfig) -> Result<Self, BufferedFeatherReaderError> {
        let file = File::open(config.path.clone())?;
        let reader = FeatherReader::new(
            file, 
            config.offset, 
            config.length
        )?;

        Ok(Self { 
            reader,
            file_id: config.file_id 
        })
    }

    /// Checks if this reader is compatible with a given file ID.
    /// 
    /// Used by the reader pool to determine if an existing reader can be reused
    /// for accessing a specific file, avoiding unnecessary reader reinitialization.
    /// 
    /// # Arguments
    /// 
    /// * `id` - File identifier to check compatibility against
    /// 
    /// # Returns
    /// 
    /// `true` if the reader is configured for the specified file, `false` otherwise.
    pub(super) fn is_compatible(&self, id: usize) -> bool {
        self.file_id == id
    }
}
