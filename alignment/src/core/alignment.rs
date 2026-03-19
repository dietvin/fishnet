/*!
 * This module handles the initial query/reference to signal alignment.
 * 
 * The AlignmentMode trait is used to match the user-specified alignment type
 * to the performed processing steps without any internal branching for more
 * efficient processing.
 * 
 * AlignmentMode is implemented for the minimal AlignQueryOnly and AlignBoth
 * structs, where AlignBoth is chosen both for keeping both alignment types
 * and only the reference alignment. The two approaches are collapsed since
 * the reference-to-signal alignment adjusts an existing query-to-signal 
 * alignment so it cannot be calculated without one.
 */
use pod5_reader_api::read::Pod5Read;
use crate::{
    core::{
        alignment::{
            aligned_read::{QueryAligned, RefAligned},
            base_read::BaseRead
        },
        loader::bam::BamRead
    },
    error::alignment_errors::AlignmentError
};

mod base_read;
mod aligned_read;
mod helpers;


/// Trait to dynamically wrap the different alignment types in one entry
/// function. It is implemented for [`AlignQueryOnly`] and [`AlignBoth`].
pub(crate) trait AlignmentMode<'a> {
    type Output;

    fn perform_alignment(read: BaseRead<'a>) -> Result<Self::Output, AlignmentError>;
}


/// Wrapper struct to propagate the alignment when 
/// only the query-to-signal alignment is required.
struct AlignQueryOnly;

impl<'a> AlignmentMode<'a> for AlignQueryOnly {
    type Output = QueryAligned<'a>;

    fn perform_alignment(read: BaseRead<'a>) -> Result<Self::Output, AlignmentError> {
        Ok(QueryAligned::from_base_read(read)?)
    }
}


/// Wrapper struct to propagate the alignment when
/// both alignments or only the reference-to-signal
/// alignment is required.
struct AlignBoth;

impl<'a> AlignmentMode<'a> for AlignBoth {
    type Output = RefAligned<'a>;

    fn perform_alignment(read: BaseRead<'a>) -> Result<Self::Output, AlignmentError> {
        let query_aligned = QueryAligned::from_base_read(read)?;
        Ok(RefAligned::from_query_aligned(query_aligned)?)
    }
}


/// Entry function for the alignment process. 
/// 
/// Initializes a BaseRead instance and propagates the alignment itself
/// depending on the chosen approach.
/// 
/// # Arguments
/// * `pod5_read` - Pod5 read to be aligned
/// * `bam_read` - Corresponding BAM read
/// * `reverse_signal` - Whether to reverse the signal for the alignment.
///                      This is required for direct RNA reads
/// 
/// # Returns
/// * `Ok(AlignmentMode::Output)` - An implemented output type. These can be
///                                 a QueryAlign or RefAlign instances, depending
///                                 on which AlignmentMode is chosen
/// * `Err(AlignmentError)` - If the BaseRead initialization or the alignment
///                           process fails
pub(crate) fn alignment<'a, A: AlignmentMode<'a>>(
    pod5_read: &Pod5Read,
    bam_read: &'a mut BamRead,
    reverse_signal: bool
) -> Result<A::Output, AlignmentError> {
    let base_read = BaseRead::new(pod5_read, bam_read, reverse_signal)?;
    Ok(A::perform_alignment(base_read)?)
}