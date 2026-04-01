/*!
    This module handles the initial query/reference to signal alignment.

    The AlignmentMode trait is used to match the user-specified alignment type
    to the performed processing steps without any internal branching for more
    efficient processing.

    AlignmentMode is implemented for the minimal AlignQueryOnly and AlignBoth
    structs, where AlignBoth is chosen both for keeping both alignment types
    and only the reference alignment. The two approaches are collapsed since
    the reference-to-signal alignment adjusts an existing query-to-signal 
    alignment so it cannot be calculated without one.
 */
use pod5_reader_api::read::Pod5Read;
use crate::{
    core::alignment::{
        aligned_read::{QueryAligned, RefAligned},
        base_read::BaseRead
    }, 
    bam::read::BamRead,
    error::core::alignment::AlignmentError
};

pub mod base_read;
pub mod aligned_read;
mod helpers;


/// Trait to dynamically wrap the different alignment types in one entry
/// function. It is implemented for [`AlignQueryOnly`] and [`AlignBoth`].
pub trait AlignmentMode: Send + Clone {
    type Output;

    /// Initialize a new instance for the alignment process.
    /// 
    /// # Arguments
    /// * `reverse_signal` - Whether to reverse the signal for the alignment.
    ///                      This is required for direct RNA reads
    /// 
    /// # Returns
    /// * Self - A new instance of a struct that implements the AlignmentMode
    ///          trait
    fn new(reverse_signal: bool) -> Self;

    /// Entry function for the alignment process. 
    /// 
    /// Initializes a BaseRead instance and propagates the alignment itself
    /// depending on the chosen approach.
    /// 
    /// # Arguments
    /// * `pod5_read` - Pod5 read to be aligned
    /// * `bam_read` - Corresponding BAM read
    /// 
    /// # Returns
    /// * `Ok(AlignmentMode::Output)` - An implemented output type. These can be
    ///                                 a QueryAlign or RefAlign instances, depending
    ///                                 on which AlignmentMode is chosen
    /// * `Err(AlignmentError)` - If the BaseRead initialization or the alignment
    ///                           process fails
    fn align(
        &self,
        pod5_read: &Pod5Read,
        bam_read: &BamRead,
    ) -> Result<Self::Output, AlignmentError>;
}


/// Wrapper struct to propagate the alignment when 
/// only the query-to-signal alignment is required.
#[derive(Clone)]
pub struct AlignQueryOnly {
    reverse_signal: bool
}

impl AlignmentMode for AlignQueryOnly {
    type Output = QueryAligned;

    fn new(reverse_signal: bool) -> Self {
        Self { reverse_signal }
    }

    fn align(
        &self,
        pod5_read: &Pod5Read,
        bam_read: &BamRead,
    ) -> Result<Self::Output, AlignmentError> {
        let base_read = BaseRead::new(
            pod5_read,
            bam_read,
            self.reverse_signal
        )?;
        Ok(QueryAligned::from_base_read(base_read)?)
    }
}


/// Wrapper struct to propagate the alignment when
/// both alignments or only the reference-to-signal
/// alignment is required.
#[derive(Clone)]
pub struct AlignBoth {
    reverse_signal: bool
}

impl AlignmentMode for AlignBoth {
    type Output = RefAligned;

    fn new(reverse_signal: bool) -> Self {
        Self { reverse_signal }
    }

    fn align(
        &self,
        pod5_read: &Pod5Read,
        bam_read: &BamRead,
    ) -> Result<Self::Output, AlignmentError> {
        let base_read = BaseRead::new(
            pod5_read,
            bam_read,
            self.reverse_signal
        )?;
        let query_aligned = QueryAligned::from_base_read(base_read)?;
        Ok(RefAligned::from_query_aligned(query_aligned)?)
    }
}