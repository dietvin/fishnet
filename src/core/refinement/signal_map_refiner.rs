/*!
 * This module provides functionality for refining the alignment between raw nanopore
 * signal data and DNA/RNA sequences. It handles the complex process of improving
 * initial alignments through iterative refinement and rescaling operations.
 * 
 * The main component is `SigMapRefiner`, which manages the refinement process for
 * both query-to-signal and reference-to-signal alignments. The refinement process
 * involves:
 * 
 * - **Initial scaling calculation**: Transforms raw signal measurements to normalized
 *   values using calibration parameters and signal statistics.
 * 
 * - **Rough rescaling**: (Optional) Applies coarse-grained scaling adjustments using 
 *   either least squares or Theil-Sen estimation to improve signal-sequence 
 *   correspondence.
 * 
 * - **Iterative refinement**: Performs multiple rounds of alignment refinement
 *   with optional rescaling between iterations to progressively improve accuracy.
 * 
 * - **Offset adjustment**: Handles signal trimming offsets from BAM record tags
 *   (sp, ts, ns) to ensure alignments correspond to the correct signal regions.
 * 
 * The module supports flexible refinement strategies through configurable settings,
 * allowing users to choose between different rescaling algorithms, iteration counts,
 * and which alignments to refine (query, reference, or both).
 * 
 * # Example Usage
 * ```ignore
 * // Create a refiner instance
 * let mut refiner = SigMapRefiner::new(
 *     &kmer_table,
 *     &mut aligned_read,
 *     &settings
 * )?;
 * 
 * // Start the refinement process
 * refiner.start()?;
 * 
 * // Access refined alignments
 * if let Some(refined_alignment) = refiner.refined_query_to_sig() {
 *     // Process refined query-to-signal alignment
 * }
 * 
 * // Get offset-adjusted alignments for use with original signal
 * let adjusted_alignment = refiner.refined_query_to_sig_offset_adjusted()?;
 * ```
 */

pub mod rescale;

use noodles::bam::Record;

use crate::core::alignment::aligned_read::AlignedRead;
use crate::core::loader::bam::BamRead;
use crate::core::loader::pod5::Pod5Read;
use crate::logger::get_log_vector_sample;
use super::kmer_table::KmerTable;
use super::settings::{RefineSettings, RoughRescaleAlgo, WhichToRefine};
use self::rescale::{rough_rescale_lstsq, rough_rescale_theil_sen, rescale};
use super::refinement_core::start_refinement::refinement;
use crate::error::refinement_errors::signal_map_refiner_errors::SigMapRefineError;

/// Structure that handles the refinement process
#[derive(Debug)]
pub struct SigMapRefiner<'a> {
    kmer_table: &'a KmerTable,
    aligned_read: &'a mut AlignedRead<'a>,
    settings: &'a RefineSettings,

    scale_dacs_to_norm: f32,
    shift_dacs_to_norm: f32,

    refined_query_to_sig: Option<Vec<usize>>,
    refined_ref_to_sig: Option<Vec<usize>>
}

impl<'a> SigMapRefiner<'a> {
    /// Initializes a new refinement instance from the path to a kmer level table,
    /// an aligned read object and settings for the refinement
    pub fn new(
        kmer_table: &'a KmerTable,
        aligned_read: &'a mut AlignedRead<'a>,
        settings: &'a RefineSettings
    ) -> Result<Self, SigMapRefineError> {
        log::info!(
            "Initializing SigMapRefiner from kmer table '{}' for read '{}'", 
            kmer_table.source_path().display(), aligned_read.read_id()
        );
        log::debug!("SigMapRefiner::new {}: Using the following settings: {:?}", aligned_read.read_id(), settings);

        // Calculate the scaling scale and shift from the 
        let (scale_dacs_to_norm, shift_dacs_to_norm) = calculate_initial_scaling_shift(
            *aligned_read.calibration_scale(),
            *aligned_read.calibration_offset(),
            aligned_read.signal_scaling_dispersion(),
            aligned_read.signal_scaling_mean()
        );

        log::debug!(
            "SigMapRefiner::new {}: scale_dacs_to_norm = {}, shift_dacs_to_norm = {}", 
            aligned_read.read_id(), scale_dacs_to_norm, shift_dacs_to_norm
        );

        Ok(SigMapRefiner {
            kmer_table,
            aligned_read,
            settings,
            scale_dacs_to_norm,
            shift_dacs_to_norm,
            refined_query_to_sig: None,
            refined_ref_to_sig: None
        })
    }

    /// Starts the refinement after initialization
    pub fn start(&mut self) -> Result<(), SigMapRefineError> {
        // Determine which alignments should be refined 
        // (query-to-signal AND/OR ref-to-signal)
        match self.settings.which_map_to_refine() {
            WhichToRefine::Query => {
                self.start_query_to_signal_refinement()?
            }
            WhichToRefine::Reference => {
                self.start_ref_to_signal_refinement()?
            }
            WhichToRefine::Both => {
                self.start_query_to_signal_refinement()?;
                self.start_ref_to_signal_refinement()?;
            }
        }

        Ok(())
    }
    
    /// Performs the refinement of the query to signal alignment
    fn start_query_to_signal_refinement(&mut self) -> Result<(), SigMapRefineError> {
        log::info!("Starting query to signal refinement for read {}", self.aligned_read.read_id());

        let signal = self.aligned_read.signal_f32()?;
        let seq_to_signal_map = self.aligned_read
            .query_to_signal()
            .ok_or(SigMapRefineError::QueryToSigNotFound)?;
        
        let sequence = self.aligned_read.query();
        let levels = self.kmer_table.extract_levels(sequence)?;

        let refined_query_to_sig: Vec<usize>;

        (refined_query_to_sig, self.scale_dacs_to_norm, self.shift_dacs_to_norm) = sequence_to_signal_refinement(
            self.scale_dacs_to_norm, 
            self.shift_dacs_to_norm, 
            seq_to_signal_map, 
            &signal, 
            &levels,
            &self.settings
        )?;

        self.refined_query_to_sig = Some(refined_query_to_sig);

        Ok(())
    }

    /// Performs the refinement of the reference to signal alignment
    fn start_ref_to_signal_refinement(&mut self) -> Result<(), SigMapRefineError> {
        log::info!("Starting reference to signal refinement for read {}", self.aligned_read.read_id());

        let signal = self.aligned_read.signal_f32()?;
        let reference_to_signal_map = self.aligned_read
            .reference_to_signal()
            .ok_or(SigMapRefineError::RefToSigNotFound)?;

        let sequence = self.aligned_read.reference()?;
        let levels = self.kmer_table.extract_levels(&sequence)?;

        let refined_reference_to_sig: Vec<usize>;

        (refined_reference_to_sig, self.scale_dacs_to_norm, self.shift_dacs_to_norm) = sequence_to_signal_refinement(
            self.scale_dacs_to_norm, 
            self.shift_dacs_to_norm, 
            reference_to_signal_map, 
            &signal, 
            &levels,
            &self.settings
        )?;

        self.refined_ref_to_sig = Some(refined_reference_to_sig);

        Ok(())
    }    

    /// Returns the refined query to signal alignment if already calculated. 
    pub fn refined_query_to_sig(&self) -> Option<&Vec<usize>> {
        self.refined_query_to_sig.as_ref()
    }

    /// Returns the refined reference to signal alignment if already calculated. 
    pub fn refined_ref_to_sig(&self) -> Option<&Vec<usize>> {
        self.refined_ref_to_sig.as_ref()
    }

    /// Returns the refined query to signal alignment if already calculated.
    /// Adjusts the boundaries by the offset given by the *sp*, *ts* and *ns* tags in 
    /// the BAM record.
    pub fn refined_query_to_sig_offset_adjusted(&self) -> Result<Option<Vec<usize>>, SigMapRefineError> {
        match &self.refined_query_to_sig {
            Some(qts) => {
                let offset = self.aligned_read.pod5_read().trimmed_signal_offset()?;
                Ok(Some(
                    qts.iter().map(|el| el + offset).collect::<Vec<usize>>()
                ))
            }
            None => Ok(None)
        }
    }

    /// Returns the refined reference to signal alignment if already calculated. 
    /// Adjusts the boundaries by the offset given by the *sp*, *ts* and *ns* tags in 
    /// the BAM record.
    pub fn refined_ref_to_sig_offset_adjusted(&self) -> Result<Option<Vec<usize>>, SigMapRefineError> {
        match &self.refined_ref_to_sig {
            Some(rts) => {
                let offset = self.aligned_read.pod5_read().trimmed_signal_offset()?;
                Ok(Some(
                    rts.iter().map(|el| el + offset).collect::<Vec<usize>>()
                ))
            }
            None => Ok(None)
        }
    }

    pub fn bam_read(&self) -> &BamRead {
        self.aligned_read.bam_read()
    }

    pub fn pod5_read(&self) -> &Pod5Read {
        self.aligned_read.pod5_read()
    }

    pub fn bam_record(&self) -> &Record {
        self.aligned_read.bam_read().get_record()
    }

    pub fn bam_record_mut(&mut self) -> &mut Record {
        self.aligned_read.bam_read_mut().get_record_mut()
    }

}

/// Calculate the scaling factor and shift to transform the raw signal measurements
/// into normalized measurements. Called during initialization
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

/// Central function to start the refinement process
/// 
/// Depending on the settings it perform rough rescaling, 
/// and the the refinement and subsequent rescaling 
/// for n iterations (set in the settings)
/// 
/// if n=0, only one round of refinement is performed without subsequent rescaling
fn sequence_to_signal_refinement(
    scale_measurements_to_norm: f32,
    shift_measurements_to_norm: f32,
    seqence_to_signal_map: &Vec<usize>,
    signal: &Vec<f32>,
    expected_levels: &Vec<f32>,
    settings: &RefineSettings
) -> Result<(Vec<usize>, f32, f32), SigMapRefineError> {
    log::debug!(
        "sequence_to_signal_refinement input: scale_measurements_to_norm = {}, shift_measurements_to_norm = {}, seqence_to_signal_map = {}, signal = {}, expected_levels = {}, settings = {:?}",
        scale_measurements_to_norm, shift_measurements_to_norm, 
        get_log_vector_sample(seqence_to_signal_map, 10), 
        get_log_vector_sample(signal, 10), 
        get_log_vector_sample(expected_levels, 10),
        settings
    );
    // Determine the rough shift and scale estimation function
    let (mut shift, mut scale) = match settings.rough_rescale_algo() {
        RoughRescaleAlgo::LeastSquares { 
            quantiles, 
            clip_bases, 
            use_base_center 
        } => {
            rough_rescale_lstsq(
                scale_measurements_to_norm,
                shift_measurements_to_norm,
                seqence_to_signal_map,
                &expected_levels,
                signal,
                quantiles,
                *clip_bases,
                *use_base_center
            )?
        }   
        RoughRescaleAlgo::TheilSen { 
            quantiles, 
            clip_bases,
            use_base_center, 
        } => {
            rough_rescale_theil_sen(
                scale_measurements_to_norm,
                shift_measurements_to_norm,
                seqence_to_signal_map,
                &expected_levels,
                signal,
                quantiles,
                *clip_bases,
                *use_base_center,
            )?
        }   
        RoughRescaleAlgo::NoRoughRescaling => (shift_measurements_to_norm, scale_measurements_to_norm) 
    };
    let mut sequence_to_signal_map_refined = seqence_to_signal_map.clone();

    let n_iterations = *settings.n_refinement_iters();
    // If the user sets n_refinement_iters to 0, one round of mapping refinement 
    // is performed without rescaling afterwards
    let perform_rescaling = n_iterations > 0;
    let n_iter = n_iterations.max(1);
    for i in 0..n_iter {
        log::debug!("sequence_to_signal_refinement: Starting refinement iteration {} of {}", i, n_iter);

        // Normalize the signal with the scaling and shift parameters
        let signal_norm = signal
            .iter()
            .map(|el| (el - shift) / scale)
            .collect::<Vec<f32>>();

        sequence_to_signal_map_refined = refinement(
            sequence_to_signal_map_refined,
            &signal_norm,
            &expected_levels,
            settings
        )?;

        if perform_rescaling {
            log::debug!("sequence_to_signal_refinement: Starting rescaling in iteration {}", i);
            (shift, scale) = rescale(
                scale,
                shift, 
                &sequence_to_signal_map_refined,
                expected_levels,
                signal,
                settings.rescale_algo()
            )?
        }
    }

    log::debug!(
        "sequence_to_signal_refinement output: sequence_to_signal_map_refined = {}, scale_dacs_to_norm = {}, shift_dacs_to_norm = {}", 
        get_log_vector_sample(&sequence_to_signal_map_refined, 10),
        scale,
        shift
    );

    Ok((sequence_to_signal_map_refined, scale, shift))
}
