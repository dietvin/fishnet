use noodles::sam::alignment::record::cigar::Op;
use pod5_reader_api::read::Pod5Read;

use crate::{core::loader::bam::BamRead, error::alignment_errors::BaseReadError};

/// Holds the data needed for the initial signal-to-sequence alignment.
/// 
/// The intended flow for BaseRead is the following:
/// `BaseRead` -> `QueryAligned` -> `RefAligned
pub(crate) struct BaseRead<'a> {
    bam_read: &'a mut BamRead,
    trimmed_signal: Vec<i16>,
    num_samples_trimmed: usize,
    signal_offset: usize,
    reverse_signal: bool
}

impl<'a> BaseRead<'a> {
    /// Initialize a new instance.
    /// 
    /// Collects needed data from a pod5 and matching bam read.
    /// Signal is extracted from the pod5 read and adjusted for
    /// the alignment process. A mutable reference to the bam
    /// read is stored in the BaseRead to avoid unnecessary
    /// cloning.
    /// 
    /// # Arguments
    /// 
    /// * `pod5_read` - Pod5 read to be aligned
    /// * `bam_read` - Corresponding BAM read
    /// * `reverse_signal` - Whether to reverse the signal for the alignment.
    ///                      This is required for direct RNA reads
    /// 
    /// # Returns
    /// 
    /// * `Ok(BaseRead)` - The newly initialized BaseRead
    /// 
    /// # Errors
    /// 
    /// * `BaseReadError::IdMismatch` - If the read IDs from the pod5 and bam
    ///                                 reads do not match
    /// * `BaseReadError::Pod5Error` - If the signal cannot be extracted from
    ///                                the pod5 read
    /// * `BaseReadError::TrimError` - If the signal trimming fails
    pub(crate) fn new(
        pod5_read: &Pod5Read, 
        bam_read: &'a mut BamRead, 
        reverse_signal: bool
    ) -> Result<Self, BaseReadError> {
        let pod5_id = pod5_read.read_id_string();
        log::info!("Initializing AlignedRead '{}'", pod5_id);
        let bam_id = bam_read.read_id();
        if pod5_id != bam_id {
            return Err(BaseReadError::IdMismatch(pod5_id.into(), bam_id.into()));
        }

        let mut signal = pod5_read.require_signal()?.clone();
        let (num_samples_trimmed, signal_offset) = Self::update_signal(
            &mut signal,
            reverse_signal,
            *bam_read.get_parent_signal_offset(), 
            *bam_read.get_trimmed_signal_length(), 
            *bam_read.get_subread_signal_length()
        )?;

        Ok(Self { 
            bam_read, 
            trimmed_signal: signal,
            num_samples_trimmed,
            signal_offset,
            reverse_signal
        })
    }

    /// Trim the signal based on the *sp*, *ts* and *ns* tags
    /// found in the corresponding bam read.
    /// 
    /// # Arguments
    /// * `reverse_signal` - bool indicating if the signal must be reversed
    /// (in case of direct RNA sequencing reads)
    /// * `parent_signal_offset` - value behind the *sp* tag if available
    /// * `trimmed_signal_len` - value behind the *ts* tag if available
    /// * `subread_signal_len` - value behind the *ns* tag if available
    /// 
    /// # Errors
    /// * `Pod5ReadError::TrimError` - If the trimming fails
    /// 
    /// # Note: 
    /// The *ts* and *ns* values are relative to the signal starting at the offset
    /// given by *sp*. Accordingly the *sp* value must be added to account for it.
    /// ```text
    /// --------------------------
    /// |   |                    |
    /// s   sp                   size
    ///     ----------------------
    ///     |    |          |    |
    ///     s_o  ts         ns
    ///          -----------
    ///         trimmed signal
    /// ```
    fn update_signal(
        signal: &mut Vec<i16>,
        reverse_signal: bool,
        parent_signal_offset: Option<usize>,
        trimmed_signal_len: Option<usize>,
        subread_signal_len: Option<usize>
    ) -> Result<(usize, usize), BaseReadError> {
        let num_samples = signal.len();
        let parent_signal_offset = match parent_signal_offset {
            Some(v) => v,
            None => 0            
        };
        let trimmed_signal_len = match trimmed_signal_len {
            Some(v) => v,
            None => 0
        };

        let start = parent_signal_offset + trimmed_signal_len;

        let end = match subread_signal_len {
            Some(v) => parent_signal_offset + v,
            None => num_samples
        };

        if end > num_samples {
            return Err(BaseReadError::TrimError(
                format!(
                    "'subread_signal_len' ({}) out of bounds with signal length {}",
                    end, num_samples
                )
            ));
        } else if start >= end {
            return Err(BaseReadError::TrimError(
                format!(
                    "Start index ({}) must be smaller than end index ({})",
                    start, end
                )
            ));
        }

        if reverse_signal {
            signal.drain(end..);
            signal.drain(..start);
            signal.reverse();
        } else {
            signal.drain(end..);
            signal.drain(..start);
        }

        log::debug!(
            "update_signal info: trimmed signal contains data from signal[{}..{}]; sig. len before = {}, after = {}",
            start, end, num_samples, signal.len()
        );

        let num_samples_trimmed = signal.len();
        // This offset will be added to the alignment(s) in the end so the alignment can be used 
        // with the signal untrimmed signal stored in the pod5 file without the tag information
        let signal_offset: usize = if reverse_signal {
            num_samples - end
        } else {
            start
        };

        Ok((num_samples_trimmed, signal_offset))
    }

    pub(super) fn bam_read(&mut self) -> &mut BamRead {
        self.bam_read
    }

    pub(super) fn trimmed_signal(&self) -> &[i16] {
        &self.trimmed_signal
    }

    pub(super) fn num_samples_trimmed(&self) -> usize {
        self.num_samples_trimmed
    }

    pub(super) fn signal_offset(&self) -> usize {
        self.signal_offset
    }

    pub(super) fn reverse_signal(&self) -> bool {
        self.reverse_signal
    }

    pub(super) fn query_length(&self) -> usize {
        self.bam_read.query_length()
    }

    pub(super) fn move_table(&self) -> &[bool] {
        self.bam_read.move_table()
    }

    pub(super) fn stride(&self) -> usize {
        self.bam_read.stride()
    }

    pub(super) fn get_reference_len(&self) -> Result<usize, BaseReadError> {
        Ok(self.bam_read.get_reference_len()?)
    }

    pub(super) fn get_cigar(&self) -> Result<&Vec<Op>, BaseReadError> {
        self.bam_read.get_cigar()?.ok_or(
            BaseReadError::CigarMissing
        )
    }
}
