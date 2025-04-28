pub mod rescale;

use crate::alignment::aligned_read::AlignedRead;
use crate::logger::get_log_vector_sample;
use super::kmer_table::KmerTable;
use super::settings::{RefineSettings, RoughRescaleAlgo, WhichToRefine};
use self::rescale::{rough_rescale_lstsq, rough_rescale_theil_sen, rescale};
use super::refinement_core::start_refinement::refinement;
use super::super::error::refinement_errors::signal_map_refiner_errors::SigMapRefineError;

/// Structure that handles the refinement process
#[derive(Debug)]
pub struct SigMapRefiner<'a> {
    kmer_table: &'a KmerTable,
    aligned_read: &'a AlignedRead<'a>,
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
        aligned_read: &'a AlignedRead<'a>,
        settings: &'a RefineSettings
    ) -> Result<Self, SigMapRefineError> {
        log::info!(
            "Initializing SigMapRefiner from kmer table '{}' for read '{}'", 
            kmer_table.source_path(), aligned_read.read_id()
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
            sequence, 
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
            sequence, 
            &signal, 
            &levels,
            &self.settings
        )?;

        self.refined_ref_to_sig = Some(refined_reference_to_sig);

        Ok(())
    }    

    /// Returns the refined query to signal alignment if already calculated. 
    /// Returns an error otherwise.
    pub fn refined_query_to_sig(&self) -> Result<&Vec<usize>, SigMapRefineError> {
        self.refined_query_to_sig.as_ref().ok_or(SigMapRefineError::RefinedQueryToSigNotFound)
    }

    /// Returns the refined reference to signal alignment if already calculated. 
    /// Returns an error otherwise.
    pub fn refined_ref_to_sig(&self) -> Result<&Vec<usize>, SigMapRefineError> {
        self.refined_ref_to_sig.as_ref().ok_or(SigMapRefineError::RefinedRefToSigNotFound)
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
    sequence: &Vec<u8>,
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
