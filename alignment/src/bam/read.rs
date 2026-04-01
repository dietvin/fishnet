/*!
    This module contains the [`BamRead`] structs.

    `BamRead` wraps read-wise sequencing and mapping information.
*/

use std::collections::HashMap;
use noodles::sam::alignment::record::cigar::Op;

use noodles::bam::record::Data;
use noodles::bam;

use crate::bam::helpers::{self, reverse_complement};
use crate::bam::ref_seq_reconstruction::build_reference_sequence;
use crate::error::bam::BamReadError;
use crate::output::schema::OutputSchema;

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
    query: Vec<u8>, // The basecalled sequence
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
    /// Creates a new BamRead from a noodles BAM record.
    ///
    /// Extracts and processes all relevant fields from the provided record,
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

    pub fn query_str(&self) -> Result<&str, BamReadError> {
        Ok(std::str::from_utf8(&self.query)?)
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
    pub fn get_reference_name(&self) -> Result<&str, BamReadError> {
        if let Some(r) = &self.reference_name {
            Ok(r.as_str())
        } else {
            Err(BamReadError::NoSuchDataForUnmappedRead("reference_name".to_string()))
        }
    }

    /// Gets the start position on the reference sequence with error handling
    ///
    /// # Returns
    ///
    /// * `Result<Option<&Vec<Cigar>>, BamReadError>` - The cigar elements, 
    /// None if the tag is not set, or an error if unmapped
    pub fn get_reference_start(&self) -> Result<usize, BamReadError> {
        self.reference_start.ok_or(
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

    pub fn get_cigar_opt(&self) -> Option<&Vec<Op>> {
        self.cigar.as_ref()
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
    pub fn get_reference_len(&self) -> Result<usize, BamReadError> {
        self.reference_len.ok_or(
            BamReadError::NoSuchDataForUnmappedRead("reference_len".to_string())
        )
    }

    pub fn get_reference_len_opt(&self) -> Option<usize> {
        self.reference_len
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

    /// Consumes itself returning the data used for output
    /// writing.
    /// Invoked in the main processing loop once the alignments
    /// were produces and get written to file.
    pub fn into_output_data<S: OutputSchema>(self) -> Result<(
        String,
        Option<String>,
        Option<usize>,
        Option<String>,
        Option<String>,
    ), BamReadError> {
        let read_id = self.read_id;

        let (ref_name, ref_start) = if S::HAS_REF_META {
            (self.reference_name, self.reference_start)
        } else {
            (None, None)
        };

        let query_seq = if S::HAS_QUERY_SEQ {
            Some(String::from_utf8(self.query)?)
        } else {
            None
        };

        let ref_seq = if S::HAS_REF_SEQ {
            Some(String::from_utf8(
                self.reference_seq.expect("Unmapped reads cannot reach this point")
            )?)
        } else {
            None
        };

        Ok((read_id, ref_name, ref_start, query_seq, ref_seq))
    }
}
