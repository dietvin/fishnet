use std::{collections::HashMap, ffi::OsString, path::PathBuf, slice::{Iter, IterMut}};

use crate::file::{Pod5File, Pod5FileError};

/// A collection of POD5 files that can be accessed as a single dataset.
/// 
/// Provides both indexed and path-based access to individual POD5 files,
/// along with iteration capabilities.
#[derive(Debug)]
pub struct Pod5Dataset {
    files: Vec<Pod5File>,
    index: HashMap<OsString, usize>,
    length: usize
}
impl Pod5Dataset {
    /// Creates a new Pod5Dataset from a list of file paths.
    /// 
    /// # Arguments
    /// * `paths` - Vector of paths to POD5 files to include in the dataset
    /// 
    /// # Returns
    /// Result containing the initialized Pod5Dataset or an error
    /// 
    /// # Errors
    /// Returns errors for invalid files or IO problems
    pub fn new(paths: &Vec<PathBuf>) -> Result<Self, Pod5DatasetError> {
        let mut files = Vec::with_capacity(paths.len()); 
        let mut index = HashMap::with_capacity(paths.len());

        for (path_idx, path_buf) in paths.iter().enumerate() {
            let path = path_buf.as_os_str().to_os_string();
            let file = Pod5File::new(path_buf.to_path_buf())?;

            files.push(file);
            index.insert(path, path_idx);
        }

        let length = files.len();
        Ok(Pod5Dataset { 
            files, 
            index,
            length
        })
    }

    /// Gets a reference to a Pod5File by its path key.
    /// 
    /// # Arguments
    /// * `key` - The path key (as OsString) of the file to retrieve
    /// 
    /// # Returns
    /// Result containing reference to the requested Pod5File or an error
    /// 
    /// # Errors
    /// Returns InvalidKey if the key doesn't exist in the dataset
    pub fn get(&self, key: &OsString) -> Result<&Pod5File, Pod5DatasetError> {
        let index = self.index.get(key).ok_or(
            Pod5DatasetError::InvalidKey(key.clone())
        )?;
        
        self.get_by_index(*index)
    }

    /// Gets a reference to a Pod5File by its index.
    /// 
    /// # Arguments
    /// * `index` - The numerical index of the file to retrieve
    /// 
    /// # Returns
    /// Result containing reference to the requested Pod5File or an error
    /// 
    /// # Errors
    /// Returns FileIndexError if the index is out of bounds
    pub fn get_by_index(&self, index: usize) -> Result<&Pod5File, Pod5DatasetError> {
        self.files.get(index).ok_or(
            Pod5DatasetError::FileIndexError(index, self.length)
        )    
    }

    /// Gets a mutable reference to a Pod5File by its path key.
    /// 
    /// # Arguments
    /// * `key` - The path key (as OsString) of the file to retrieve
    /// 
    /// # Returns
    /// Result containing mutable reference to the requested Pod5File or an error
    /// 
    /// # Errors
    /// Returns InvalidKey if the key doesn't exist in the dataset
    pub fn get_mut(&mut self, key: &OsString) -> Result<&mut Pod5File, Pod5DatasetError> {
        let index = self.index.get(key).ok_or(
            Pod5DatasetError::InvalidKey(key.clone())
        )?;
        
        self.get_by_index_mut(*index)
    }

    /// Gets a mutable reference to a Pod5File by its index.
    /// 
    /// # Arguments
    /// * `index` - The numerical index of the file to retrieve
    /// 
    /// # Returns
    /// Result containing mutable reference to the requested Pod5File or an error
    /// 
    /// # Errors
    /// Returns FileIndexError if the index is out of bounds
    pub fn get_by_index_mut(&mut self, index: usize) -> Result<&mut Pod5File, Pod5DatasetError> {
        self.files.get_mut(index).ok_or(
            Pod5DatasetError::FileIndexError(index, self.length)
        )    
    }

    /// Returns a vector of references to all Pod5Files in the dataset.
    pub fn files(&self) -> Vec<&Pod5File> {
        self.files.iter().collect()
    }

    /// Returns an iterator over references to all Pod5Files in the dataset.
    pub fn iter_files(&self) -> Iter<'_, Pod5File> {
        self.files.iter()
    }

    /// Returns a mutable iterator over all Pod5Files in the dataset.
    pub fn iter_files_mut(&mut self) -> IterMut<'_, Pod5File> {
        self.files.iter_mut()
    }

    /// Returns the number of files in the dataset.
    pub fn len(&self) -> usize {
        self.length
    }
}


/// Error type for Pod5Dataset operations.
/// 
/// Includes variants for:
/// - Underlying Pod5File errors
/// - Invalid key access
/// - Index out of bounds errors
#[derive(Debug, thiserror::Error)]
pub enum Pod5DatasetError {
    #[error("Pod5File error: {0}")]
    Pod5FileError(#[from] Pod5FileError),
    #[error("Key {0:?} not found in dataset")]
    InvalidKey(OsString),
    #[error("File index out of bounds: {0} (len={1})")]
    FileIndexError(usize, usize),
    #[error("Read index out of bounds: {0} (len={1})")]
    ReadIndexError(usize, usize),
}