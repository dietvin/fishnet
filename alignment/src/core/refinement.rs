/*!
    Signal-to-sequence refinement module.

    This module implements an iterative refinement procedure that improves an
    initial sequence-to-signal alignment using:

    1. Signal normalization (scale/shift estimation)
    2. Banded dynamic programming (DP)
    3. Iterative rescaling between refinement steps

    The design is fully generic over:

    * Rough rescaling strategy (`RoughRescaleAlgo`)
    * Iterative rescaling strategy (`RescaleAlgo`)
    * Core refinement / DP algorithm (`RefinementAlgo`)

    The refinement process operates on an initial alignment produced by the
    alignment stage (e.g. `QueryAligned` or `RefAligned`) and returns refined
    mappings from sequence coordinates to signal indices.

    # Architecture

    The entry point abstraction is [`RefinementMode`], which:

    * Defines the expected input alignment type via a GAT (`Input`)
    * Defines the output type (`Output`)
    * Encapsulates configuration and algorithm choices

    Concrete implementations:

    * [`RefineQueryToSignal`] – refines query → signal mapping
    * [`RefineRefToSignal`] – refines reference → signal mapping
    * [`RefineBoth`] – refines both mappings in a single pass

    All implementations delegate to [`refine_alignment`], which contains the
    core iterative refinement loop.

    # Key Properties

    * Zero runtime dispatch (fully monomorphized)
    * Compile-time enforcement of alignment/refinement compatibility
    * Pluggable algorithmic components
*/

use kmer_table::kmer_table::KmerTable;
use pod5_reader_api::read::Pod5Read;
use crate::{
    bam::read::BamRead, core:: {
        alignment::aligned_read::{QueryAligned, RefAligned}, 
        refinement::{
            band::sequence_band::SequenceBand,
            dp::{banded_db, forward_step::RefinementAlgo},
            rescaling::{RescaleAlgo, rescale},
            rough_rescaling::RoughRescaleAlgo
        }
    },
    error::core::refinement::RefinementError,
    output::record::{BothResult, QueryToSignalResult, RefToSignalResult},
};

pub mod rough_rescaling;
pub mod rescaling;
pub mod dp;
pub mod band;

/// Abstraction over refinement strategies operating on aligned reads.
///
/// This trait decouples:
///
/// * The **input alignment representation** (`Input`)
/// * The **refinement algorithm configuration**
/// * The **output mapping type** (`Output`)
///
/// It is designed to be composed with [`AlignmentMode`] such that:
///
/// ```text
/// RefinementMode::Input == AlignmentMode::Output
/// ```
///
/// ensuring type-safe pipeline composition at compile time.
///
/// # Type Parameters
///
/// * `S: RoughRescaleAlgo`  
///   Algorithm used for initial coarse rescaling before refinement iterations.
///
/// * `T: RescaleAlgo`  
///   Algorithm used for iterative rescaling between refinement steps.
///
/// * `U: RefinementAlgo`  
///   Core dynamic programming algorithm used to update the alignment.
///
/// # Associated Types
///
/// * `Input`  
///   Alignment representation consumed by the refinement stage.
///   One of `QueryAligned` or `RefAligned`.
///
/// * `Output`  
///   Final refined mapping:
///   * `Vec<usize>` for single mappings
///   * `(Vec<usize>, Vec<usize>)` for dual mappings
pub trait RefinementMode<
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo
>: Clone + Send {
    type Input;
    type Output;

    fn new(
        n_refinement_iter: usize,
        half_bandwidth: usize,
        is_banded: bool,
        min_step: usize,
        rough_rescale_algo: S,
        rescale_algo: T,
        refinement_algo: U
    ) -> Self;

    fn refine(
        &self,
        aligned_read: Self::Input,
        kmer_table: &KmerTable,
        pod5_read: &Pod5Read,
        bam_read: &BamRead,
    ) -> Result<Self::Output, RefinementError>;
}


/// Refinement mode for query-to-signal alignment.
///
/// This mode:
///
/// * Consumes a [`QueryAligned`] input
/// * Refines the `query_to_signal` mapping
/// * Produces a single `Vec<usize>` mapping
///
/// # Behavior
///
/// The refinement is performed using the shared iterative procedure
/// implemented in [`refine_alignment`], operating on:
///
/// * The raw signal extracted from the read
/// * Expected signal levels derived from the query sequence
///
/// This is the minimal refinement mode when only query coordinates
/// are required.
#[derive(Clone)]
pub struct RefineQueryToSignal<
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo
> {
    n_refinement_iter: usize,
    half_bandwidth: usize,
    is_banded: bool,
    min_step: usize,
    rough_rescale_algo: S,
    rescale_algo: T,
    refinement_algo: U
}

impl<S, T, U> RefinementMode<S, T, U> for RefineQueryToSignal<S, T, U> 
where
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo
{
    type Input = QueryAligned;
    type Output = QueryToSignalResult;

    fn new(
        n_refinement_iter: usize,
        half_bandwidth: usize,
        is_banded: bool,
        min_step: usize,
        rough_rescale_algo: S,
        rescale_algo: T,
        refinement_algo: U
    ) -> Self {
        Self { 
            n_refinement_iter,
            half_bandwidth,
            is_banded,
            min_step,
            rough_rescale_algo,
            rescale_algo,
            refinement_algo
        }
    }

    fn refine(
        &self,
        aligned_read: Self::Input,
        kmer_table: &KmerTable,
        pod5_read: &Pod5Read,
        bam_read: &BamRead,
    ) -> Result<Self::Output, RefinementError> {
        let query_to_sig = aligned_read.query_to_signal;
        let trimmed_signal = aligned_read.base.signal_f32();
        let trimmed_signal_offset = aligned_read.base.signal_offset();

        let query_to_sig = refine_alignment(
            query_to_sig,
            bam_read.query(),

            pod5_read.require_calibration_scale()?,
            pod5_read.require_calibration_offset()?,
            bam_read.signal_scaling_dispersion(),
            bam_read.signal_scaling_mean(),

            &trimmed_signal,
            trimmed_signal_offset,
            kmer_table,
            &self.rough_rescale_algo,
            &self.rescale_algo,
            &self.refinement_algo,
            self.n_refinement_iter,
            self.half_bandwidth,
            self.is_banded,
            self.min_step,
        )?;

        Ok(QueryToSignalResult { query_to_sig })
    }
}


/// Refinement mode for reference-to-signal alignment.
///
/// This mode:
///
/// * Consumes a [`RefAligned`] input
/// * Refines the `ref_to_signal` mapping
/// * Produces a single `Vec<usize>` mapping
///
/// # Behavior
///
/// The refinement operates on the reference projection of the alignment,
/// which is derived from an existing query alignment.
///
/// This mode is used when only reference coordinates are required.
#[derive(Clone)]
pub struct RefineRefToSignal<
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo
> {
    n_refinement_iter: usize,
    half_bandwidth: usize,
    is_banded: bool,
    min_step: usize,
    rough_rescale_algo: S,
    rescale_algo: T,
    refinement_algo: U
}

impl<S, T, U> RefinementMode<S, T, U> for RefineRefToSignal<S, T, U> 
where
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo
{
    type Input = RefAligned;
    type Output = RefToSignalResult;

    fn new(
        n_refinement_iter: usize,
        half_bandwidth: usize,
        is_banded: bool,
        min_step: usize,
        rough_rescale_algo: S,
        rescale_algo: T,
        refinement_algo: U
    ) -> Self {
        Self {
            n_refinement_iter,
            half_bandwidth,
            is_banded,
            min_step,
            rough_rescale_algo,
            rescale_algo,
            refinement_algo
        }
    }

    fn refine(
        &self,
        aligned_read: Self::Input,
        kmer_table: &KmerTable,
        pod5_read: &Pod5Read,
        bam_read: &BamRead
    ) -> Result<Self::Output, RefinementError> {
        let ref_to_sig = aligned_read.ref_to_signal;
        let trimmed_signal = aligned_read.base.signal_f32();
        let trimmed_signal_offset = aligned_read.base.signal_offset();

        let ref_to_sig = refine_alignment(
            ref_to_sig,
            bam_read.get_reference()?,

            pod5_read.require_calibration_scale()?,
            pod5_read.require_calibration_offset()?,
            bam_read.signal_scaling_dispersion(),
            bam_read.signal_scaling_mean(),

            &trimmed_signal,
            trimmed_signal_offset,
            kmer_table,
            &self.rough_rescale_algo,
            &self.rescale_algo,
            &self.refinement_algo,
            self.n_refinement_iter,
            self.half_bandwidth,
            self.is_banded,
            self.min_step,
        )?;

        Ok(RefToSignalResult { ref_to_sig })
    }
}


/// Refinement mode that produces both query-to-signal and reference-to-signal mappings.
///
/// This mode:
///
/// * Consumes a [`RefAligned`] input
/// * Independently refines using identical settings:
///   * `query_to_signal`
///   * `ref_to_signal`
/// * Returns both mappings as a tuple
///
/// # Output
///
/// ```text
/// (query_to_signal, ref_to_signal)
/// ```
#[derive(Clone)]
pub struct RefineBoth<
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo,
> {
    n_refinement_iter: usize,
    half_bandwidth: usize,
    is_banded: bool,
    min_step: usize,
    rough_rescale_algo: S,
    rescale_algo: T,
    refinement_algo: U
}

impl<S, T, U> RefinementMode<S, T, U> for RefineBoth<S, T, U> 
where
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo
{
    type Input = RefAligned;
    type Output = BothResult;

    fn new(
        n_refinement_iter: usize,
        half_bandwidth: usize,
        is_banded: bool,
        min_step: usize,
        rough_rescale_algo: S,
        rescale_algo: T,
        refinement_algo: U
    ) -> Self {
        Self {
            n_refinement_iter,
            half_bandwidth,
            is_banded,
            min_step,
            rough_rescale_algo,
            rescale_algo,
            refinement_algo
        }
    }

    fn refine(
        &self,
        aligned_read: Self::Input,
        kmer_table: &KmerTable,
        pod5_read: &Pod5Read,
        bam_read: &BamRead
    ) -> Result<Self::Output, RefinementError> {
        let trimmed_signal = aligned_read.base.signal_f32();
        let trimmed_signal_offset = aligned_read.base.signal_offset();

        let query_to_sig = aligned_read.query_to_signal;
        let ref_to_sig = aligned_read.ref_to_signal;
    
        let query_to_sig = refine_alignment(
            query_to_sig,
            bam_read.query(),

            pod5_read.require_calibration_scale()?,
            pod5_read.require_calibration_offset()?,
            bam_read.signal_scaling_dispersion(),
            bam_read.signal_scaling_mean(),

            &trimmed_signal,
            trimmed_signal_offset,
            kmer_table,
            &self.rough_rescale_algo,
            &self.rescale_algo,
            &self.refinement_algo,
            self.n_refinement_iter,
            self.half_bandwidth,
            self.is_banded,
            self.min_step,
        )?;

        let ref_to_sig = refine_alignment(
            ref_to_sig,
            bam_read.get_reference()?,

            pod5_read.require_calibration_scale()?,
            pod5_read.require_calibration_offset()?,
            bam_read.signal_scaling_dispersion(),
            bam_read.signal_scaling_mean(),

            &trimmed_signal,
            trimmed_signal_offset,
            kmer_table,
            &self.rough_rescale_algo,
            &self.rescale_algo,
            &self.refinement_algo,
            self.n_refinement_iter,
            self.half_bandwidth,
            self.is_banded,
            self.min_step,
        )?;

        Ok(BothResult { query_to_sig, ref_to_sig })
    }
}


/// Core iterative refinement routine for sequence-to-signal mappings.
///
/// This function performs multiple rounds of alignment refinement using:
///
/// 1. Initial scaling estimation
/// 2. Rough rescaling
/// 3. Iterative banded dynamic programming
/// 4. Optional rescaling between iterations
///
/// # Arguments
///
/// * `seq_to_signal_map` - Initial mapping from sequence indices to signal indices
/// * `signal` - Raw (trimmed) signal values
/// * `kmer_table` - Provides expected signal levels for sequence kmers
/// * `pod5_read` - Source of calibration parameters
/// * `bam_read` - Provides sequence and scaling metadata
/// * `rough_rescale_algo` - Initial rescaling strategy
/// * `rescale_algo` - Iterative rescaling strategy
/// * `refinement_algo` - Core DP algorithm
/// * `n_refinement_iter` - Number of refinement iterations
/// * `band_half_bandwidth` - Half the bandwidth of the restricted DP
/// * `band_is_banded` - Whether to apply banding constraints
/// * `band_min_step` - Minimum step between one base and the next to enforce in band adjustment
/// 
/// # Algorithm
///
/// 1. Extract expected signal levels from the sequence
/// 2. Compute initial `(scale, shift)`
/// 3. Apply rough rescaling
/// 4. Normalize mapping to start at zero (for stable band construction)
/// 5. Iterate:
///    * Normalize signal using current `(scale, shift)`
///    * Construct a [`SequenceBand`]
///    * Run banded DP (`banded_db`)
///    * Update mapping
///    * Re-estimate `(scale, shift)` (except final iteration)
/// 6. Restore original coordinate offset
/// 
/// # Returns
///
/// Refined sequence-to-signal mapping.
///
/// # Errors
///
/// Returns [`SigMapRefineError`] if:
///
/// * K-mer level extraction fails
/// * Band construction fails
/// * DP or rescaling steps fail
fn refine_alignment<
    'a, 
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo
>(
    mut seq_to_signal_map: Vec<usize>,
    sequence: &Vec<u8>,

    // Arguments used to calculate initial scale & shift
    calibration_scale: f32,
    calibration_offset: f32,
    signal_scaling_dispersion: f32,
    signal_scaling_mean: f32,

    trimmed_signal: &[f32],
    trimmed_signal_offset: usize,
    kmer_table: &KmerTable,
    rough_rescale_algo: &S,
    rescale_algo: &T,
    refinement_algo: &U,
    n_refinement_iter: usize,
    band_half_bandwidth: usize,
    band_is_banded: bool,
    band_min_step: usize
) -> Result<Vec<usize>, RefinementError> {
    let levels = kmer_table.extract_levels(sequence)?;

    let (mut scale, mut shift) = calculate_initial_scaling_shift(
        calibration_scale,
        calibration_offset,
        signal_scaling_dispersion,
        signal_scaling_mean
    );

    (scale, shift) = rough_rescale_algo.rough_rescale(
        scale,
        shift,
        &seq_to_signal_map,
        &levels,
        &trimmed_signal
    )?;

    let seq_to_signal_map_start = seq_to_signal_map[0];
    let seq_to_signal_map_end = seq_to_signal_map[seq_to_signal_map.len() - 1];

    // The start and end indices of the alginment should never change
    // so the sequence-to-signal map zeroing can be done once before
    // the loop. The zeroing is reversed only once at the end

    // Allocate the memory for the normalized signal, the band and 
    let mut signal_norm: Vec<f32>;
    let mut band: SequenceBand;

    for i in 0..n_refinement_iter {
        seq_to_signal_map
            .iter_mut()
            .for_each(|el| *el -= seq_to_signal_map_start);

        band = SequenceBand::new(
            &seq_to_signal_map,
            levels.len(),
            band_half_bandwidth,
            band_is_banded,
            band_min_step
        )?;

        signal_norm = trimmed_signal[seq_to_signal_map_start..seq_to_signal_map_end]
            .iter()
            .map(|el| (el - shift) / scale)
            .collect::<Vec<f32>>();

        seq_to_signal_map = banded_db(
            &signal_norm,
            &levels,
            &band,
            refinement_algo
        );

        seq_to_signal_map
            .iter_mut()
            .for_each(|el| *el += seq_to_signal_map_start);

        // Skip rescaling in last iteration
        if i < n_refinement_iter - 1 {
            (scale, shift) = rescale(
                scale,
                shift,
                &seq_to_signal_map,
                &trimmed_signal,
                &levels,
                rescale_algo
            )?;
        }
    }

    seq_to_signal_map
        .iter_mut()
        .for_each(|el| *el += trimmed_signal_offset);

    Ok(seq_to_signal_map)
}


/// Computes the initial linear transformation parameters for signal normalization.
///
/// The transformation combines device-level calibaration and read-level normalization
/// parameters to convert raw signal measurements into normalized space:
/// ```text
/// normalized = (raw - shift) / scale
/// ```
/// It serves as the starting point for subsequent refinement-driven rescaling.
/// 
/// # Inputs
///
/// * `calibration_scale` - Device calibration scaling factor
/// * `calibration_offset` - calibration offset
/// * `scale_pa_to_norm` - scaling from picoampere space to normalized space
/// * `shift_pa_to_norm` - shift from picoampere space to normalized space
///
/// # Returns
///
/// `(scale, shift)` used for initial normalization.
fn calculate_initial_scaling_shift(
    calibration_scale: f32,
    calibration_offset: f32,
    scale_pa_to_norm: f32,
    shift_pa_to_norm: f32
) -> (f32, f32) {
    // Calculate the scale to transform raw measurements to normalized measurements
    let scale_measurements_to_pa = 1.0 / calibration_scale;
    let scale_measurements_to_norm = scale_measurements_to_pa * scale_pa_to_norm;

    // Calculate the shift to transform raw measurements to normalized measurements
    let shift_measurements_to_norm = scale_measurements_to_pa * shift_pa_to_norm - calibration_offset;

    (scale_measurements_to_norm, shift_measurements_to_norm)
}
