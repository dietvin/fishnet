/*!
 * BAM file processing and lazy-loading module for nanopore sequencing data.
 * 
 * This module provides efficient handling of BAM files containing nanopore sequencing 
 * alignments with specialized signal-level information. It extracts essential alignment 
 * data while maintaining access to nanopore-specific tags and metadata.
 * 
 * Key components:
 * - `BamRead`: Streamlined BAM record representation optimized for signal-sequence alignment
 * - `BamFileLazy`: Indexed BAM file reader with random access by read ID
 * - Support for nanopore-specific tags (move tables, signal scaling, parent read info)
 * - Efficient reference sequence reconstruction from CIGAR and MD strings
 * - Lazy-loading architecture to minimize memory usage for large BAM files
 * 
 * Features:
 * - Extracts move tables and stride information for signal-to-base mapping
 * - Handles both forward and reverse complement alignments
 * - Processes signal scaling parameters (sm/sd tags) for normalization
 * - Manages parent read relationships and signal offsets for subreads
 * - Provides O(1) random access to BAM records after initial indexing
 */

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use noodles::sam::Header;
use noodles::sam::alignment::record::cigar::Op;

use noodles::bam::record::Data;
use noodles::bam;
use noodles::bgzf;

use crate::error::loader_errors::bam_errors::{BamFileError, BamReadError};
use crate::core::loader::helpers::{self, reverse_complement};
use crate::core::loader::ref_seq_reconstruction::build_reference_sequence;

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
    reference_name: Option<String>,
    reference_start: Option<usize>,
    cigar: Option<Vec<Op>>,
    reference_seq: Option<Vec<u8>>,
    reference_len: Option<usize>,
    reverse_mapped: Option<bool>,
    parent_read_id: Option<String>, // stored in the pi tag
    parent_signal_offset: Option<usize>, // stored in the sp tag, start position in parent
    trimmed_signal_length: Option<usize>, // stored in the ts tag
    subread_signal_length: Option<usize>, // stored in the ns tag

    record: bam::Record,
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
    pub fn new(
        bam_record: bam::Record,
        ref_seq_index: &HashMap<usize, String>
    ) -> Result<Self, BamReadError> {
        let bam_data = bam_record.data();
        let bam_flags = bam_record.flags();

        let read_id = bam_record.name()
            .map(|rn| rn.to_string())
            .ok_or(BamReadError::ReadIdError)?;

        log::info!("Initializing BamRead '{}'", read_id);

        let mut query= bam_record.sequence().iter().collect::<Vec<u8>>();
        let query_length = query.len();

        let (stride, move_table): (usize, Vec<bool>) = BamRead::get_stride_move_table(&bam_data)?;

        let sm_tag = helpers::get_float_tag(&bam_data, "sm")?;
        let sd_tag = helpers::get_float_tag(&bam_data, "sd")?;

        let mapped = !bam_flags.is_unmapped();

        let mut reference_name: Option<String> = None;
        let mut reference_start: Option<usize> = None;
        let mut cigar: Option<Vec<Op>> = None; 
        let mut reference_seq: Option<Vec<u8>> = None;
        let mut reference_len: Option<usize> = None;
        let mut reverse_mapped: Option<bool> = None;

        let pi_tag = helpers::unpack_tag(
            helpers::get_str_tag(&bam_data, "pi"),
            None
        )?;
        let sp_tag = helpers::unpack_tag(
            helpers::get_uint_tag(&bam_data, "sp"),
            Some(0 as usize)
        )?;
        let ts_tag = helpers::unpack_tag(
            helpers::get_uint_tag(&bam_data, "ts"),
            Some(0 as usize)
        )?;
        let ns_tag = helpers::unpack_tag(
            helpers::get_uint_tag(&bam_data, "ns"),
            None
        )?;

        if mapped {
            let ref_seq_key = bam_record
                .reference_sequence_id()
                .ok_or_else(|| BamReadError::RefNameNotFound)??;

            reference_name = Some(ref_seq_index
                .get(&ref_seq_key)
                .cloned()
                .ok_or(BamReadError::InvalidRefSeqKey(ref_seq_key))?
            );

            reference_start = Some(bam_record
                .alignment_start()
                .ok_or(BamReadError::ReferenceStartNotFound)??
                .get()
            );
            let mut cigar_raw = bam_record.cigar().iter()
                .collect::<Result<Vec<Op>, std::io::Error>>()
                .map_err(|e| BamReadError::CigarError(e))?;
            
            let md_string = helpers::get_str_tag(&bam_data, "MD")?;
            let reference_seq_raw = build_reference_sequence(&query, &cigar_raw, &md_string.as_bytes())?;
            reference_len = Some(reference_seq_raw.len());

            let is_reverse = bam_flags.is_reverse_complemented();
            if is_reverse {
                query = reverse_complement(&query)?;

                reference_seq = Some(reverse_complement(&reference_seq_raw)?);
                cigar_raw.reverse();
                cigar = Some(cigar_raw);
            } else {
                reference_seq = Some(reference_seq_raw);
                cigar = Some(cigar_raw);
            }

            reverse_mapped = Some(is_reverse);
        }

        log::debug!(
            "BamRead::new info: read id = {}; is mapped = {}; query length = {}, reference length = {:?}", 
            read_id, mapped, query_length, reference_len
        );

        Ok(BamRead {
            read_id,
            query,
            query_length,
            move_table,
            stride,
            signal_scaling_mean: sm_tag,
            signal_scaling_dispersion: sd_tag,
            mapped,
            reference_name,
            reference_start,
            cigar,
            reference_seq,
            reference_len,
            reverse_mapped,
            parent_read_id: pi_tag,
            parent_signal_offset: sp_tag,
            trimmed_signal_length: ts_tag,
            subread_signal_length: ns_tag,
            record: bam_record
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
    fn get_stride_move_table(bam_data: &Data) -> Result<(usize, Vec<bool>), BamReadError> {
        let mv_table = helpers::get_iarray_tag(bam_data, "mv")?;
    
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
    pub fn query(&self) -> &Vec<u8> {
        &self.query
    }

    /// Gets the query length
    ///
    /// # Returns
    ///
    /// * `usize` - The length of the query sequence
    pub fn query_length(&self) -> usize {
        self.query_length
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

    /// Gets the name of the reference sequence with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&Vec<Cigar>>, BamReadError>` - The cigar elements, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_reference_name(&self) -> Result<&String, BamReadError> {
        self.reference_name.as_ref().ok_or(
            BamReadError::NoSuchDataForUnmappedRead("reference_name".to_string())
        )
    }

    /// Gets the start position on the reference sequence with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&Vec<Cigar>>, BamReadError>` - The cigar elements, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_reference_start(&self) -> Result<&usize, BamReadError> {
        self.reference_start.as_ref().ok_or(
            BamReadError::NoSuchDataForUnmappedRead("reference_start".to_string())
        )
    }

    /// Gets the CIGAR string with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&Vec<Cigar>>, BamReadError>` - The cigar elements, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_cigar(&self) -> Result<Option<&Vec<Op>>, BamReadError> {
        if self.mapped {
            Ok(self.cigar.as_ref())
        } else {
            Err(BamReadError::NoSuchDataForUnmappedRead("cigar".to_string()))
        }
    }

    /// Gets the reference sequence with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&Vec<Cigar>>, BamReadError>` - The reference sequence,
    /// or an error if unmapped
    pub fn get_reference(&self) -> Result<&Vec<u8>, BamReadError> {
        self.reference_seq.as_ref().ok_or(
            BamReadError::NoSuchDataForUnmappedRead("reference_seq".to_string())
        )
    }

    /// Gets the reference sequence length with error handling
    ///
    /// # Returns
    ///
    /// * `Result<&usize, BamReadError>` - The length of the reference sequence, 
    /// or an error if unmapped
    pub fn get_reference_len(&self) -> Result<&usize, BamReadError> {
        self.reference_len.as_ref().ok_or(
            BamReadError::NoSuchDataForUnmappedRead("reference_len".to_string())
        )
    }

    /// Gets the parent read ID with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&str>, BamReadError>` - The parent read id, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_parent_read_id(&self) -> Result<Option<&str>, BamReadError> {
        if self.mapped {
            Ok(self.parent_read_id.as_deref())
        } else {
            Err(BamReadError::NoSuchDataForUnmappedRead("parent_read_id".to_string()))
        }
    }

    /// Gets the parent signal offset
    ///
    /// # Returns
    ///
    /// * `&Option<usize` - The parent signal offset, 
    /// None if the tag is not set
    pub fn get_parent_signal_offset(&self) -> &Option<usize> {
        &self.parent_signal_offset
    }

    /// Gets the trimmed signal length
    ///
    /// # Returns
    ///
    /// * `&Option<usize>` - The trimmed signal length, 
    /// None if the tag is not set
    pub fn get_trimmed_signal_length(&self) -> &Option<usize> {
        &self.trimmed_signal_length
    }

    /// Gets the subread signal length
    ///
    /// # Returns
    ///
    /// * `Option<&usize>` - The subread signal length, 
    /// None if the tag is not set
    pub fn get_subread_signal_length(&self) -> &Option<usize> {
        &self.subread_signal_length
    }

    /// Gets a reference to the underlying read record
    ///
    /// # Returns
    ///
    /// * `&Record` - The original record from which the BamRead was constructed
    pub fn get_record(&self) -> &bam::Record {
        &self.record
    }

    /// Gets a mutable reference tothe underlying read record
    ///
    /// # Returns
    ///
    /// * `&mut Record` - The original record from which the BamRead was constructed
    pub fn get_record_mut(&mut self) -> &mut bam::Record {
        &mut self.record
    }
}




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


