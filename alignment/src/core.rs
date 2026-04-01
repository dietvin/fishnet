use kmer_table::kmer_table::KmerTable;
use pod5_reader_api::read::Pod5Read;

use crate::{
    bam::read::BamRead,
    core::{
        alignment::AlignmentMode,
        refinement::{
            RefinementMode,
            dp::forward_step::RefinementAlgo,
            rescaling::RescaleAlgo,
            rough_rescaling::RoughRescaleAlgo
        }
    },
    error::core::AlignmentCoreError, output::{
        record::IntoOutputRecord,
        schema::OutputSchema
    }
};

pub mod alignment;
pub mod refinement;


/// Executes the full alignment to refinement pipeline in a fully generic,
/// zero-cost abstraction over alignment and refinement strategies.
/// 
/// This function composes two independent stages:
///
/// 1. **Alignment stage (`A`)**
///    Produces an intermediate alignment representation (`A::Output`),
///    either [`QueryAligned`] or [`RefAligned`].
///
/// 2. **Refinement stage (`R`)**
///    Consumes the alignment output and refines it into the final mapping
///    (`R::Output`), e.g. a `Vec<usize>` or `(Vec<usize>, Vec<usize>)`.
///
/// The key type constraint is:
///
/// ```text
/// R::Input<'a> = A::Output
/// ```
/// This enforces at compile time that the chosen refinement strategy is
/// compatible with the chosen alignment mode. Invalid combinations are
/// rejected statically.
/// 
/// This design uses static dispatch and associated type contstraints instead of runtime
/// branching to eliminate internal enum matching in the main processing loop of a worker
/// thread.
/// 
/// # Type parameters
/// 
/// * `'a` - Lifetime tying the alignment output to the borrowed [`BamRead`]
/// * `A: AlignmentMode<'a>` - Defines how the initial alignment is performed.
///                            Determines the intermediate representation via
///                            `A::Output`.
/// * `S: RoughRescaleAlgo` - Algorithm used for initial rescaling parameter
///                           estimation at the start of the refinement
/// * `T: RescaleAlgo` - Algorithm used for iterative rescale parameter
///                      calculation during refinement
/// * `U: RefineAlgo` - Core dynamic programming alignment algorithm used during
///                     refinement
/// * `R: RefinementMode<S, T, U>` - Defines how refinement is performed and what
///                                 output is produced. Must accept the alignment
///                                 output as input: `R::Input<'a> = A::Output`.
/// 
/// # Arguments
/// 
/// * `pod5_read` - POD5 read to be aligned
/// * `bam_read` - Matching BAM read (borrowed for lifetime 'a)
/// * `kmer_table` - Kmer levels table mapping sequence kmers to expected signal levels
/// * `alignment_mode` - Alignment strategy controlling the produced alignment representation
/// * `refinement_mode` - Refinement strategy controlling how the alignment is refined
/// 
/// # Returns
/// 
/// * `Ok(R::Output)` - Final refined mapping:
///     - `Vec<usize>` for a single alignment (query-to-signal / ref-to-signal)
///     - `(Vec<usize>, Vec<usize>)` when both mappings are produced
/// * `Err(AlignmentError)` - If either the alignment or refinement fails
pub(crate) fn run_alignment<A, S, T, U, R, OS> (
    pod5_read: &Pod5Read,
    bam_read: &BamRead,
    kmer_table: &KmerTable,
    alignment_mode: &A,
    refinement_mode: &R
) -> Result<R::Output, AlignmentCoreError> 
where
    A: AlignmentMode,
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo,
    R: RefinementMode<S, T, U, Input = A::Output>,
    R::Output: IntoOutputRecord<OS>,
    OS: OutputSchema
{
    // Can be a QueryAligned or RefAligned instance
    let aligned_read = alignment_mode.align(
        &pod5_read,
        &bam_read
    )?;

    // Can be a Vec<usize> (if only one is aligned) or (Vec<usize>, Vec<usize>)
    // (if both reference and query are aligned)
    let refined_alignment = refinement_mode.refine(
        aligned_read,
        kmer_table,
        &pod5_read,
        &bam_read
    )?;

    Ok(refined_alignment)
}