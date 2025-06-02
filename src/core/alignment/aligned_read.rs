use crate::error::alignment_errors::AlignedReadError;
use crate::logger::get_log_vector_sample;

use super::super::loader::{bam::BamRead, pod5::Pod5Read};
use super::{query_to_signal, reference_to_signal};

/// Represents a nanopore read with associated alignment information.
///
/// This struct combines raw signal data from a Pod5 file with alignment information
/// from a BAM file, providing functionality to map between reference sequence,
/// query sequence, and raw signal positions.
#[derive(Debug)]
pub struct AlignedRead<'a> {
    pod5_read: &'a mut Pod5Read,
    bam_read: &'a mut BamRead,
    reverse_signal: bool,
    query_to_signal: Option<Vec<usize>>,
    reference_to_signal: Option<Vec<usize>>
}

impl<'a> AlignedRead<'a> {
    /// Creates a new AlignedRead by combining Pod5 and BAM read data.
    ///
    /// # Arguments
    ///
    /// * `pod5_read` - The Pod5 read containing raw signal data
    /// * `bam_read` - The BAM read containing alignment information
    /// * `reverse_signal` - Whether the signal should be reversed (true for direct RNA data)
    ///
    /// # Returns
    ///
    /// * `Ok(AlignedRead)` - The combined read data
    /// * `Err(AlignedReadError)` - If there's an issue combining the reads
    ///
    /// # Errors
    ///
    /// * `AlignedReadError::IdMismatch` - If the read IDs in the Pod5 and BAM files don't match
    /// * Other errors if signal updating fails
    pub fn new(pod5_read: &'a mut Pod5Read, bam_read: &'a mut BamRead, reverse_signal: bool) -> Result<Self, AlignedReadError> {
        let pod5_id = pod5_read.read_id();
        log::info!("Initializing AlignedRead '{}'", pod5_id);

        let bam_id = bam_read.read_id();
        if pod5_id != bam_id {
            return Err(AlignedReadError::IdMismatch(pod5_id.to_string(), bam_id.to_string()));
        }

        pod5_read.update_signal(
            reverse_signal, 
            *bam_read.get_parent_signal_offset(), 
            *bam_read.get_trimmed_signal_length(), 
            *bam_read.get_subread_signal_length()
        )?;

        Ok(AlignedRead{
            pod5_read,
            bam_read,
            reverse_signal,
            query_to_signal: None,
            reference_to_signal: None
        })
    }

    /// Computes the mapping from query sequence positions to signal positions.
    ///
    /// This method uses the move table from the BAM file to determine which signal
    /// positions correspond to bases in the query sequence.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the mapping was successfully computed
    /// * `Err(AlignedReadError)` - If there was an error computing the mapping
    pub fn align_query_to_signal(&mut self) -> Result<(), AlignedReadError> {
        log::info!("Aligning query to signal '{}'", self.read_id());

        self.query_to_signal = Some(
            query_to_signal::align_query_to_signal(
                self.bam_read.move_table(),
                self.bam_read.stride(),
                *self.pod5_read.num_samples_trimmed()?,
                self.reverse_signal,
                self.bam_read.query_length()
            )?
        );

        log::debug!(
            "align_query_to_signal info: read id = {}; alignment = {}", 
            self.read_id(), get_log_vector_sample(self.query_to_signal().unwrap(), 10)
        );

        Ok(())
    }

    /// Computes the mapping from reference sequence positions to signal positions.
    ///
    /// This method uses the CIGAR string from the BAM file and the pre-computed
    /// query-to-signal mapping to determine which signal positions correspond to
    /// bases in the reference sequence.
    ///
    /// # Requirements
    ///
    /// * The read must be mapped to a reference sequence
    /// * The query-to-signal mapping must already be computed
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the mapping was successfully computed
    /// * `Err(AlignedReadError)` - If there was an error computing the mapping
    ///
    /// # Errors
    ///
    /// * `AlignedReadError::Unmapped` - If the read is not mapped to a reference
    /// * `AlignedReadError::RefBeforeQuery` - If query-to-signal mapping hasn't been computed
    /// * Other errors if the reference-to-signal mapping calculation fails
    pub fn align_reference_to_signal(&mut self) -> Result<(), AlignedReadError> {
        log::info!("Aligning reference to signal '{}'", self.read_id());

        if !self.bam_read.is_mapped() {
            return Err(
                AlignedReadError::Unmapped
            );
        } else if let Some(query_to_signal) = self.query_to_signal() {
            // No else here because these can not be None if the is_mapped check passes 
            if let (
                Some(cigar), 
                ref_len
            ) = (
                    self.bam_read.get_cigar()?, 
                    self.bam_read.get_reference_len()?
                ) {
                self.reference_to_signal = Some(
                    reference_to_signal::align_reference_to_signal(
                        cigar, 
                        query_to_signal, 
                        *ref_len
                    )?
                );
            }
        } else {
            return Err(AlignedReadError::RefBeforeQuery);
        }

        log::debug!(
            "align_reference_to_signal info: read id = {}; alignment = {}", 
            self.read_id(), get_log_vector_sample(self.reference_to_signal().unwrap(), 10)
        );

        Ok(())
    }

    /// Gets the computed query-to-signal mapping.
    ///
    /// # Returns
    ///
    /// * `Some(&Vec<usize>)` - The mapping vector if it has been computed
    /// * `None` - If the mapping hasn't been computed yet
    pub fn query_to_signal(&self) -> Option<&Vec<usize>> {
        self.query_to_signal.as_ref()
    }

    /// Gets the computed reference-to-signal mapping.
    ///
    /// # Returns
    ///
    /// * `Some(&Vec<usize>)` - The mapping vector if it has been computed
    /// * `None` - If the mapping hasn't been computed yet
    pub fn reference_to_signal(&self) -> Option<&Vec<usize>> {
        self.reference_to_signal.as_ref()
    }

    /// Gets a reference to the Pod5 read.
    ///
    /// # Returns
    ///
    /// * `&Pod5Read` - The Pod5 read containing raw signal data
    pub fn pod5_read(&self) -> &Pod5Read {
        &self.pod5_read
    }

    /// Returns the unique identifier from the underlying Pod5Read
    pub fn read_id(&self) -> &str {
        self.pod5_read.read_id()
    }

    /// Returns the signal intensity values from the underlying Pod5Read
    pub fn signal(&self) -> &Vec<i16> {
        self.pod5_read.signal()
    }

    /// Returns the trimmed signal intensity values from the underlying 
    /// Pod5Read as f32
    pub fn signal_f32(&self) -> Result<Vec<f32>, AlignedReadError> {
        Ok(
            self.pod5_read.signal_trimmed()?
                .iter()
                .map(|el| *el as f32)
                .collect::<Vec<f32>>()
        )
    }

    /// Returns the number of samples from the trimmed signal
    /// of the underlying Pod5Read
    pub fn num_samples(&self) -> &usize {
        self.pod5_read.num_samples()
    }

    /// Returns the calibration offset from the underlying Pod5Read
    pub fn calibration_offset(&self) -> &f32 {
        self.pod5_read.calibration_offset()
    }

    /// Returns the calibration scale factor from the underlying Pod5Read
    pub fn calibration_scale(&self) -> &f32 {
        self.pod5_read.calibration_scale()
    }

    /// Gets a reference to the BAM read.
    ///
    /// # Returns
    ///
    /// * `&BamRead` - The BAM read containing alignment information
    pub fn bam_read(&self) -> &BamRead {
        &self.bam_read
    }

    /// Gets a mutable reference to the BAM read.
    ///
    /// # Returns
    ///
    /// * `&BamRead` - The BAM read containing alignment information
    pub fn bam_read_mut(&mut self) -> &mut BamRead {
        &mut self.bam_read
    }


    /// Returns the query sequence as bytes from the underlying BamRead
    pub fn query(&self) -> &Vec<u8> {
        self.bam_read.query()
    }

    /// Returns the query length from the underlying BamRead
    pub fn query_length(&self) -> usize {
        self.bam_read.query().len()
    }


    /// Returns the move table from the underlying BamRead
    pub fn move_table(&self) -> &[bool] {
        self.bam_read.move_table()
    }

    /// Returns the stride value from the underlying BamRead
    pub fn stride(&self) -> usize {
        self.bam_read.stride()
    }

    /// Returns the signal scaling mean (sm tag) from the underlying BamRead
    pub fn signal_scaling_mean(&self) -> f32 {
        self.bam_read.signal_scaling_mean()
    }

    /// Returns the signal scaling dispersion (sd tag) from the underlying BamRead
    pub fn signal_scaling_dispersion(&self) -> f32 {
        self.bam_read.signal_scaling_dispersion()
    }

    /// Returns whether the the underlying BamRead is mapped 
    pub fn is_mapped(&self) -> bool {
        self.bam_read.is_mapped()
    }

    /// Returns whether the the underlying BamRead 
    pub fn reference(&self) -> Result<&Vec<u8>, AlignedReadError> {
        Ok(self.bam_read.get_reference()?)
    }

    pub fn reference_len(&self) -> Result<&usize, AlignedReadError> {
        Ok(self.bam_read.get_reference_len()?)
    }

}