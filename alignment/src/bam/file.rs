/*!
    This module contains the [`BamFileLazy`] struct.

    `BamFileLazy` parses a provided BAM file to set up an internal index
    that allows for random access to individual reads without the need to
    load the entire file into memory.
*/

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use noodles::sam::Header;
use noodles::bam;
use noodles::bgzf;

use crate::bam::read::BamRead;
use crate::error::bam::BamFileError;


/// A lazy-loading BAM file reader with random access by read ID
///
/// This struct provides indexed access to BAM records, building an in-memory
/// index mapping read IDs to file offsets for efficient retrieval.
pub struct BamFileLazy {
    path: PathBuf,
    bam_reader: bam::io::Reader<bgzf::io::Reader<File>>,
    index: HashMap<String, bgzf::VirtualPosition>,
    ref_sequence_index: HashMap<usize, String>,
    header: Header
}


impl BamFileLazy {
    /// Creates a new BamFileLazy by indexing all records in the given BAM file
    ///
    /// Scans through the entire BAM file once to build an in-memory index of
    /// read IDs to file offsets, enabling future random access.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the BAM file
    ///
    /// # Returns
    ///
    /// * `Result<Self, BamFileError>` - A new BamFileLazy instance or an error
    ///
    /// # Note
    ///
    /// This operation can be expensive for large BAM files as it requires a full scan.
    pub fn new(path: &PathBuf) -> Result<Self, BamFileError> {
        log::info!("Initializing BamFileLazy from file '{}'", path.display());

        // Initialize a bam Reader wrapping a bgzf Reader in order to store offsets 
        // of the contained reads 
        let file = File::open(path)?;
        let buf_reader = bgzf::io::Reader::new(file);
        let mut bam_reader = bam::io::Reader::from(buf_reader);
        // Extract the header and skip to the start of the alignments
        let header = bam_reader.read_header()?;

        // Extract the reference dictionary to get the ref seq names later on 
        let ref_sequence_index = header
            .reference_sequences()
            .keys()
            .enumerate()
            .map(|(i, name)| (i, name.to_string()))
            .collect();

        // Initialize the index storing the offset for each read in a hashmap
        let mut index: HashMap<String, bgzf::VirtualPosition> = HashMap::new();

        loop {
            let offset = {
                let inner = bam_reader.get_ref();
                inner.virtual_position()
            };
    
            let mut record = bam::Record::default();
    
            let n = bam_reader.read_record(&mut record)?;
    
            if n == 0 {
                break;
            }
    
            if let Some(name) = record.name().map(|rn| rn.to_string()) {
                index.entry(name).or_insert(offset);
            }
        }

        log::debug!("BamFileLazy::new info: path = {}, #reads = {}", path.display(), index.len());
        Ok(BamFileLazy {
            path: path.clone(),
            bam_reader,
            index,
            ref_sequence_index,
            header
        })
    }

    /// Retrieves a BAM record by its read ID
    ///
    /// Uses the pre-built index to seek directly to the specified record and parse it.
    ///
    /// # Arguments
    ///
    /// * `id` - The read ID to retrieve
    ///
    /// # Returns
    ///
    /// * `Result<BamRead, BamFileError>` - The requested BAM record or an error
    ///
    /// # Errors
    ///
    /// * `BamFileError::IndexError` - If the read ID is not found in the index
    /// * `BamFileError::ValueError` - If the record cannot be read after seeking
    pub fn get(&mut self, id: &str) -> Result<BamRead, BamFileError> {
        log::info!("Loading BamRead '{}'", id);
        let offset = *self.index.get(id).ok_or(
            BamFileError::IndexError(String::from(id))
        )?;

        self.bam_reader.get_mut().seek(offset)?;

        let mut record = bam::Record::default();
        let n = self.bam_reader.read_record(&mut record)?;

        if n == 0 {
            return Err(BamFileError::ValueError(
                "Block size 0, index corresponds to EOF.".to_string()
            ));
        }

        // Double check that the read id is the one that is wanted
        if let Some(name) = record.name().map(|rn| rn.to_string()) {
            if name == id {
                let bam_read = BamRead::new(
                    record, 
                    &self.ref_sequence_index
                )?;
                Ok(bam_read)
            } else {
                Err(BamFileError::ReadIdMismatch(name, id.to_string()))
            } 
        } else {
            Err(BamFileError::RecordAccessError)
        }
    }

    /// Gets a reference to the internal read ID to file offset index
    ///
    /// # Returns
    ///
    /// * `&HashMap<String, i64>` - Reference to the index HashMap
    pub fn index(&self) -> &HashMap<String, bgzf::VirtualPosition> {
        &self.index
    }

    /// Gets the path to the BAM file
    ///
    /// # Returns
    ///
    /// * `&str` - Path to the BAM file
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Gets the number of indexed records
    ///
    /// # Returns
    ///
    /// * `usize` - Number of records in the index
    pub fn len(&self) -> usize {
        self.index.len()
    }
    
    /// Checks if the index is empty
    ///
    /// # Returns
    ///
    /// * `bool` - True if no records are indexed, false otherwise
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
    
    /// Checks if a read ID exists in the index
    ///
    /// # Arguments
    ///
    /// * `id` - The read ID to check
    ///
    /// # Returns
    ///
    /// * `bool` - True if the read ID exists, false otherwise
    pub fn contains(&self, id: &str) -> bool {
        self.index.contains_key(id)
    }
    
    /// Gets all read IDs in the index
    ///
    /// # Returns
    ///
    /// * `Vec<&String>` - Vector of all read IDs
    pub fn read_ids(&self) -> Vec<&String> {
        self.index.keys().collect()
    }

    /// Reopen the BAM reader if it has been closed or encountered an error
    ///
    /// # Returns
    ///
    /// * `Result<(), BamFileError>` - Success or an error
    pub fn reopen(&mut self) -> Result<(), BamFileError> {
        let file = File::open(&self.path)?;
        let buf_reader = bgzf::io::Reader::new(file);
        let mut bam_reader = bam::io::Reader::from(buf_reader);
        bam_reader.read_header()?;

        self.bam_reader = bam_reader;
        Ok(())
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
}


