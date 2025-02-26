use std::collections::HashMap;
use rust_htslib::bam::{record::CigarString, Record, Reader, Read};
use super::super::errors::bam_errors::{BamReadError, BamFileError};
use super::helpers;

/// Represents a BAM record with specialized fields for sequencing data.
/// The BAM record is stripped down to only the information that is needed
/// for the signal to sequence alignment.
/// 
/// This struct encapsulates both common BAM data and specialized fields
/// for signal-level information, including optional fields available only
/// for mapped reads.
#[derive(Debug)]
pub struct BamRead {
    read_id: String,
    query: Vec<u8>,
    move_table: Vec<bool>,
    stride: u16,
    signal_scaling_mean: f32, // stored in the sm tag
    signal_scaling_dispersion: f32, // stored in the sd tag
    
    mapped: bool,
    // The following data is only available if a read is mapped
    cigar: Option<CigarString>,
    parent_read_id: Option<String>, // stored in the pi tag
    parent_signal_offset: Option<usize>, // stored in the sp tag, start position in parent
    trimmed_signal_length: Option<usize>, // stored in the ts tag
    subread_signal_length: Option<usize> // stored in the ns tag
}

impl BamRead {
    /// Creates a new BamRead from a BAM record
    ///
    /// Extracts and processes all relevant fields from the provided BAM record,
    /// handling both common fields and optional fields for mapped reads.
    ///
    /// # Arguments
    ///
    /// * `bam_record` - The BAM record to process
    ///
    /// # Returns
    ///
    /// * `Result<Self, BamReadError>` - A new BamRead instance or an error
    pub fn new(bam_record: Record) -> Result<Self, BamReadError> {
        let read_id = std::str::from_utf8(bam_record.qname())?.to_string();
        let query = bam_record.seq().as_bytes();

        let (stride, move_table): (u16, Vec<bool>) = BamRead::get_stride_move_table(&bam_record)?;

        let sm_tag = helpers::get_float_tag(&bam_record, "sm")?;
        let sd_tag = helpers::get_float_tag(&bam_record, "sd")?;

        let mapped = !bam_record.is_unmapped();
        let mut cigar = None; 
        
        let mut pi_tag = None; 
        let mut sp_tag = None; 
        let mut ts_tag = None; 
        let mut ns_tag = None; 

        if mapped {
            cigar = Some(bam_record.cigar().take());
            pi_tag = helpers::unpack_tag(
                helpers::get_str_tag(&bam_record, "pi"),
                None
            )?;
            sp_tag = helpers::unpack_tag(
                helpers::get_uint_tag(&bam_record, "sp"),
                Some(0 as usize)
            )?;
            ts_tag = helpers::unpack_tag(
                helpers::get_uint_tag(&bam_record, "ts"),
                Some(0 as usize)
            )?;
            ns_tag = helpers::unpack_tag(
                helpers::get_uint_tag(&bam_record, "ns"),
                None
            )?;
        }

        Ok(BamRead {
            read_id,
            query,
            move_table,
            stride,
            signal_scaling_mean: sm_tag,
            signal_scaling_dispersion: sd_tag,
            mapped,
            cigar,
            parent_read_id: pi_tag,
            parent_signal_offset: sp_tag,
            trimmed_signal_length: ts_tag,
            subread_signal_length: ns_tag,
        })
    }

    /// Extracts stride and move table information from a BAM record
    ///
    /// Processes the 'mv' tag to determine stride and create a boolean move table.
    ///
    /// # Arguments
    ///
    /// * `bam_record` - The BAM record to extract data from
    ///
    /// # Returns
    ///
    /// * `Result<(u16, Vec<bool>), BamReadError>` - The stride and move table, or an error
    fn get_stride_move_table(bam_record: &Record) -> Result<(u16, Vec<bool>), BamReadError> {
        let mv_table = helpers::get_iarray_tag(bam_record, "mv")?;
    
        let stride = mv_table[0] as u16;
        let move_table = mv_table[1..].iter().map(|&el| el != 0).collect::<Vec<bool>>();
    
        Ok((
            stride,
            move_table
        ))
    }

    /// Gets the read identifier
    ///
    /// # Returns
    ///
    /// * `&str` - The read identifier
    pub fn read_id(&self) -> &str {
        &self.read_id
    }

    /// Gets the query sequence
    ///
    /// # Returns
    ///
    /// * `&[u8]` - The query sequence as bytes
    pub fn query(&self) -> &[u8] {
        &self.query
    }

    /// Gets the move table
    ///
    /// # Returns
    ///
    /// * `&[bool]` - The move table as a slice of booleans
    pub fn move_table(&self) -> &[bool] {
        &self.move_table
    }

    /// Gets the stride value
    ///
    /// # Returns
    ///
    /// * `u16` - The stride value
    pub fn stride(&self) -> u16 {
        self.stride
    }

    /// Gets the signal scaling mean
    ///
    /// # Returns
    ///
    /// * `f32` - The signal scaling mean (from sm tag)
    pub fn signal_scaling_mean(&self) -> f32 {
        self.signal_scaling_mean
    }

    /// Gets the signal scaling dispersion
    ///
    /// # Returns
    ///
    /// * `f32` - The signal scaling dispersion (from sd tag)
    pub fn signal_scaling_dispersion(&self) -> f32 {
        self.signal_scaling_dispersion
    }

    /// Checks if the read is mapped
    ///
    /// # Returns
    ///
    /// * `bool` - True if the read is mapped, false otherwise
    pub fn is_mapped(&self) -> bool {
        self.mapped
    }

    /// Gets the CIGAR string if available
    ///
    /// # Returns
    ///
    /// * `Option<&CigarString>` - The CIGAR string or None if unmapped
    pub fn cigar(&self) -> Option<&CigarString> {
        self.cigar.as_ref()
    }

    /// Gets the parent read ID if available
    ///
    /// # Returns
    ///
    /// * `Option<&str>` - The parent read ID or None if unmapped or no parent
    pub fn parent_read_id(&self) -> Option<&str> {
        self.parent_read_id.as_deref()
    }

    /// Gets the parent signal offset if available
    ///
    /// # Returns
    ///
    /// * `Option<usize>` - The parent signal offset or None if unmapped
    pub fn parent_signal_offset(&self) -> Option<usize> {
        self.parent_signal_offset
    }

    /// Gets the trimmed signal length if available
    ///
    /// # Returns
    ///
    /// * `Option<usize>` - The trimmed signal length or None if unmapped
    pub fn trimmed_signal_length(&self) -> Option<usize> {
        self.trimmed_signal_length
    }

    /// Gets the subread signal length if available
    ///
    /// # Returns
    ///
    /// * `Option<usize>` - The subread signal length or None if unmapped or unavailable
    pub fn subread_signal_length(&self) -> Option<usize> {
        self.subread_signal_length
    }

    /// Gets the CIGAR string with error handling
    ///
    /// # Returns
    ///
    /// * `Result<&CigarString, BamReadError>` - The CIGAR string or an error if unmapped
    pub fn get_cigar(&self) -> Result<&CigarString, BamReadError> {
        self.cigar.as_ref().ok_or(BamReadError::NoSuchDataForUnmappedRead)
    }

    /// Gets the parent read ID with error handling
    ///
    /// # Returns
    ///
    /// * `Result<&str, BamReadError>` - The parent read ID or an error if unmapped/unavailable
    pub fn get_parent_read_id(&self) -> Result<&str, BamReadError> {
        self.parent_read_id.as_deref().ok_or(BamReadError::NoSuchDataForUnmappedRead)
    }

    /// Gets the parent signal offset with error handling
    ///
    /// # Returns
    ///
    /// * `Result<usize, BamReadError>` - The parent signal offset or an error if unmapped
    pub fn get_parent_signal_offset(&self) -> Result<usize, BamReadError> {
        self.parent_signal_offset.ok_or(BamReadError::NoSuchDataForUnmappedRead)
    }

    /// Gets the trimmed signal length with error handling
    ///
    /// # Returns
    ///
    /// * `Result<usize, BamReadError>` - The trimmed signal length or an error if unmapped
    pub fn get_trimmed_signal_length(&self) -> Result<usize, BamReadError> {
        self.trimmed_signal_length.ok_or(BamReadError::NoSuchDataForUnmappedRead)
    }

    /// Gets the subread signal length with error handling
    ///
    /// # Returns
    ///
    /// * `Result<usize, BamReadError>` - The subread signal length or an error if unmapped/unavailable
    pub fn get_subread_signal_length(&self) -> Result<usize, BamReadError> {
        self.subread_signal_length.ok_or(BamReadError::NoSuchDataForUnmappedRead)
    }
}


/// A lazy-loading BAM file reader with random access by read ID
///
/// This struct provides indexed access to BAM records, building an in-memory
/// index mapping read IDs to file offsets for efficient retrieval.
#[derive(Debug)]
pub struct BamFileLazy {
    path: String,
    bam_reader: Reader,
    index: HashMap<String, i64>
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
    pub fn new(path: &str) -> Result<Self, BamFileError> {
        let mut bam = Reader::from_path(path)?;
        let mut index: HashMap<String, i64> = HashMap::new();

        let mut offset = bam.tell();
        while let Some(read) = bam.records().next() {
            let read = read?;
            let id = std::str::from_utf8(read.qname())?;
            index.insert(String::from(id), offset);

            offset = bam.tell();
        }

        Ok(BamFileLazy { 
            path: String::from(path), 
            bam_reader: bam, 
            index 
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
        let offset = *self.index.get(id).ok_or(
            BamFileError::IndexError(String::from(id))
        )?;

        self.bam_reader.seek(offset)?;

        match self.bam_reader.records().next() {
            Some(record) => {
                let record = record?;
                let bam_read = BamRead::new(record)?;
                Ok(bam_read)
            },
            None => Err(BamFileError::ValueError(String::from(id)))
        }
    }

    /// Gets a reference to the internal read ID to file offset index
    ///
    /// # Returns
    ///
    /// * `&HashMap<String, i64>` - Reference to the index HashMap
    pub fn index(&self) -> &HashMap<String, i64> {
        &self.index
    }

    /// Gets the path to the BAM file
    ///
    /// # Returns
    ///
    /// * `&str` - Path to the BAM file
    pub fn path(&self) -> &str {
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
        self.bam_reader = Reader::from_path(&self.path)?;
        Ok(())
    }
}


