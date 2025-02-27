use super::super::loader::{bam::BamRead, pod5::Pod5Read};
use super::super::error::alignment_errors::aligned_read_errors::AlignedReadError;
use super::{query_to_signal, reference_to_signal};

pub struct AlignedRead<'a> {
    pod5_read: &'a mut Pod5Read,
    bam_read: &'a BamRead,
    reverse_signal: bool,
    query_to_signal: Option<Vec<usize>>,
    reference_to_signal: Option<Vec<usize>>
}

impl<'a> AlignedRead<'a> {
    pub fn new(pod5_read: &'a mut Pod5Read, bam_read: &'a BamRead, reverse_signal: bool) -> Result<Self, AlignedReadError> {
        let pod5_id = pod5_read.read_id();
        let bam_id = bam_read.read_id();
        if pod5_id != bam_id {
            return Err(AlignedReadError::IdMismatch(pod5_id.to_string(), bam_id.to_string()));
        }

        pod5_read.update_signal(
            reverse_signal, 
            bam_read.parent_signal_offset(), 
            bam_read.trimmed_signal_length(), 
            bam_read.subread_signal_length()
        )?;

        Ok(AlignedRead{
            pod5_read,
            bam_read,
            reverse_signal,
            query_to_signal: None,
            reference_to_signal: None
        })
    }

    pub fn align_query_to_signal(&mut self) -> Result<(), AlignedReadError> {
        self.query_to_signal = Some(
            query_to_signal::align_query_to_signal(
                self.bam_read.move_table(),
                self.bam_read.stride(),
                *self.pod5_read.num_samples(),
                self.reverse_signal,
                self.bam_read.query_length()
            )?
        );
        Ok(())
    }

    pub fn align_reference_to_signal(&mut self) -> Result<(), AlignedReadError> {
        if !self.bam_read.is_mapped() {
            return Err(
                AlignedReadError::Unmapped
            );
        } else if let Some(query_to_signal) = self.query_to_signal() {
            // No else here because these can not be None if the is_mapped check passes 
            if let (
                Some(cigar), 
                Some(rev_mapped), 
                Some(ref_len)) = (
                    self.bam_read.cigar(), 
                    self.bam_read.is_reverse_mapped(), 
                    self.bam_read.reference_len()
                ) {
                self.reference_to_signal = Some(
                    reference_to_signal::align_reference_to_signal(
                        cigar, 
                        query_to_signal, 
                        *rev_mapped, 
                        *ref_len
                    )?
                );
            }
        } else {
            return Err(AlignedReadError::RefBeforeQuery);
        }
        Ok(())
    }

    pub fn query_to_signal(&self) -> Option<&Vec<usize>> {
        self.query_to_signal.as_ref()
    }
    pub fn reference_to_signal(&self) -> Option<&Vec<usize>> {
        self.reference_to_signal.as_ref()
    }
}