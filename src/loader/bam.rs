use std::collections::HashMap;
use rust_htslib::bam::ext::BamRecordExtensions;
use rust_htslib::bam::{record::Cigar, Record, Reader, Read};
use super::super::error::loader_errors::bam_errors::{BamReadError, BamFileError};
use super::helpers;

// ##################################################################################################
// #                                            Structs                                             #
// ##################################################################################################


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
    query_length: usize,
    move_table: Vec<bool>,
    stride: usize,
    signal_scaling_mean: f32, // stored in the sm tag
    signal_scaling_dispersion: f32, // stored in the sd tag
    
    mapped: bool,
    // The following data is only available if a read is mapped
    cigar: Option<Vec<Cigar>>,
    reference_len: Option<usize>,
    reverse_mapped: Option<bool>,
    parent_read_id: Option<String>, // stored in the pi tag
    parent_signal_offset: Option<usize>, // stored in the sp tag, start position in parent
    trimmed_signal_length: Option<usize>, // stored in the ts tag
    subread_signal_length: Option<usize> // stored in the ns tag
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


// ##################################################################################################
// #                                        Implementations                                         #
// ##################################################################################################

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
        let query_length = query.len();
        let (stride, move_table): (usize, Vec<bool>) = BamRead::get_stride_move_table(&bam_record)?;

        let sm_tag = helpers::get_float_tag(&bam_record, "sm")?;
        let sd_tag = helpers::get_float_tag(&bam_record, "sd")?;

        let mapped = !bam_record.is_unmapped();
        let mut cigar = None; 
        let mut reference_len = None;
        let mut reverse_mapped = None;
        let mut pi_tag = None; 
        let mut sp_tag = None; 
        let mut ts_tag = None; 
        let mut ns_tag = None; 

        if mapped {
            cigar = Some(bam_record.cigar().take().0);
            reference_len = Some(
                (bam_record.reference_end() - bam_record.reference_start()) as usize
            );
            reverse_mapped = Some(bam_record.is_reverse());
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
            query_length,
            move_table,
            stride,
            signal_scaling_mean: sm_tag,
            signal_scaling_dispersion: sd_tag,
            mapped,
            cigar,
            reference_len,
            reverse_mapped,
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
    fn get_stride_move_table(bam_record: &Record) -> Result<(usize, Vec<bool>), BamReadError> {
        let mv_table = helpers::get_iarray_tag(bam_record, "mv")?;
    
        let stride = mv_table[0] as usize;
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

    /// Gets the query length
    ///
    /// # Returns
    ///
    /// * `usize` - The length of the query sequence
    pub fn query_length(&self) -> usize {
        self.query.len()
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
    pub fn stride(&self) -> usize {
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

    /// Determines if the read is reverse mapped
    ///
    /// # Returns
    ///
    /// * `Option<&usize>` - True if reverse mapped, false otherwise or None if unmapped
    pub fn is_reverse_mapped(&self) -> Option<&bool> {
        self.reverse_mapped.as_ref()
    }

    /// Gets the CIGAR string with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&Vec<Cigar>>, BamReadError>` - The cigar elements, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_cigar(&self) -> Result<Option<&Vec<Cigar>>, BamReadError> {
        if self.mapped {
            Ok(self.cigar.as_ref())
        } else {
            Err(BamReadError::NoSuchDataForUnmappedRead("cigar".to_string()))
        }
    }

    /// Gets the reference sequence length with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&usize>, BamReadError>` - The length of the reference sequence, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_reference_len(&self) -> Result<Option<&usize>, BamReadError> {
        if self.mapped {
            Ok(self.reference_len.as_ref())
        } else {
            Err(BamReadError::NoSuchDataForUnmappedRead("reference_len".to_string()))
        }
    }

    /// Gets the parent read ID with error handling
    ///
    /// # Returns
    ///
    /// * `Result<&str, BamReadError>` - The parent read id, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_parent_read_id(&self) -> Result<Option<&str>, BamReadError> {
        if self.mapped {
            Ok(self.parent_read_id.as_deref())
        } else {
            Err(BamReadError::NoSuchDataForUnmappedRead("parent_read_id".to_string()))
        }
    }

    /// Gets the parent signal offset with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&usize>, BamReadError>` - The parent signal offset, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_parent_signal_offset(&self) -> Result<Option<usize>, BamReadError> {
        if self.mapped {
            Ok(self.parent_signal_offset)
        } else {
            Err(BamReadError::NoSuchDataForUnmappedRead("parent_signal_offset".to_string()))
        }
    }

    /// Gets the trimmed signal length with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&usize>, BamReadError>` - The trimmed signal length, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_trimmed_signal_length(&self) -> Result<Option<usize>, BamReadError> {
        if self.mapped {
            Ok(self.trimmed_signal_length)
        } else {
            Err(BamReadError::NoSuchDataForUnmappedRead("trimmed_signal_length".to_string()))
        }
    }

    /// Gets the subread signal length with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&usize>, BamReadError>` - The subread signal length, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_subread_signal_length(&self) -> Result<Option<usize>, BamReadError> {
        if self.mapped {
            Ok(self.subread_signal_length)
        } else {
            Err(BamReadError::NoSuchDataForUnmappedRead("subread_signal_length".to_string()))
        }
    }
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


