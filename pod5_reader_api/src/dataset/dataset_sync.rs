use std::{collections::HashMap, ffi::OsString, path::PathBuf, slice::{Iter, IterMut}};
use uuid::Uuid;
use crate::{dataset::Pod5Dataset, error::dataset::Pod5DatasetError, file::Pod5File, read::Pod5Read};

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
        let mut file_index = HashMap::with_capacity(paths.len());
        let mut reads_index = HashMap::new();
        for (path_idx, path_buf) in paths.iter().enumerate() {
            let path = path_buf.as_os_str().to_os_string();
            let file = Pod5File::new(&path_buf.to_path_buf())?;

            // Keep track on which read is in which file
            let read_ids = file.read_ids().clone();
            for read_id in read_ids {
                reads_index.insert(read_id, path_idx);
            }

            files.push(file);
            file_index.insert(path, path_idx);
        }

        let n_files = files.len();
        let n_reads = reads_index.len();
        Ok(Pod5Dataset { 
            files, 
            file_index,
            n_files,
            reads_index,
            n_reads,        
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
    pub fn get_file(&self, key: &OsString) -> Result<&Pod5File, Pod5DatasetError> {
        let index = self.file_index.get(key).ok_or(
            Pod5DatasetError::InvalidKey(key.clone())
        )?;
        
        self.get_file_by_index(*index)
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
    pub fn get_file_by_index(&self, index: usize) -> Result<&Pod5File, Pod5DatasetError> {
        self.files.get(index).ok_or(
            Pod5DatasetError::FileIndexError(index, self.n_files)
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
    pub fn get_file_mut(&mut self, key: &OsString) -> Result<&mut Pod5File, Pod5DatasetError> {
        let index = self.file_index.get(key).ok_or(
            Pod5DatasetError::InvalidKey(key.clone())
        )?;
        
        self.get_file_by_index_mut(*index)
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
    pub fn get_file_by_index_mut(&mut self, index: usize) -> Result<&mut Pod5File, Pod5DatasetError> {
        self.files.get_mut(index).ok_or(
            Pod5DatasetError::FileIndexError(index, self.n_files)
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
    pub fn n_files(&self) -> usize {
        self.n_files
    }

    pub fn get_read(&mut self, read_id: &Uuid) -> Result<Pod5Read, Pod5DatasetError> {
        let file_idx = self.reads_index.get(read_id)
            .ok_or(Pod5DatasetError::ReadIdNotFound(*read_id))?;

        let file = self.get_file_by_index_mut(*file_idx)?;
        let read = file.get(read_id)?;
        Ok(read)
    }

    pub fn n_reads(&self) -> usize {
        self.n_reads
    }
}