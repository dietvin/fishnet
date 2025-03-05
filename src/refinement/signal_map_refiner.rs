mod bands;
mod rescale;
pub mod settings;

use crate::alignment::aligned_read;

use settings::{RefineSettings, RefineAlgo, RoughRescaleAlgo, RescaleAlgo, WhichToRefine};
use super::kmer_table::KmerTable;
use self::rescale::{rough_rescale_lstsq, rough_rescale_theil_sen, rescale_lstsq, rescale_theil_sen};
use super::super::alignment::aligned_read::AlignedRead;
use super::super::error::refinement_errors::signal_map_refiner_errors::SigMapRefineError;

#[derive(Debug)]
pub struct SigMapRefiner<'a> {
    kmer_table: KmerTable,
    aligned_read: &'a AlignedRead<'a>,
    settings: RefineSettings,

    scale_dacs_to_norm: f32,
    shift_dacs_to_norm: f32,

    refined_query_to_sig: Option<Vec<usize>>,
    refined_ref_to_sig: Option<Vec<usize>>
}

impl<'a> SigMapRefiner<'a> {
    pub fn new(
        kmer_table_path: &str,
        aligned_read: &'a AlignedRead<'a>,
        settings: RefineSettings
    ) -> Result<Self, SigMapRefineError> {
        let mut kmer_table = KmerTable::new(kmer_table_path)?;
        if *settings.normalize_levels() {
            kmer_table.fix_gauge()?;
        }

        let (scale_dacs_to_norm, shift_dacs_to_norm) = calculate_scaling_shift(
            *aligned_read.calibration_scale(),
            *aligned_read.calibration_offset(),
            aligned_read.signal_scaling_mean(),
            aligned_read.signal_scaling_dispersion()
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

    pub fn start(&mut self) -> Result<(), SigMapRefineError> {
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
    
    fn start_query_to_signal_refinement(&mut self) -> Result<(), SigMapRefineError> {
        let signal = self.aligned_read.signal();
        let seq_to_signal_map = self.aligned_read
            .query_to_signal()
            .ok_or(SigMapRefineError::QueryToSigNotFound)?;
        
        let seq = self.aligned_read.query();
        let levels = self.kmer_table.extract_levels(seq)?;

        self.perform_rough_rescaling(
            signal,
            seq_to_signal_map,
            &levels
        )?;
        Ok(())
        
    }

    fn start_ref_to_signal_refinement(&mut self) -> Result<(), SigMapRefineError> {
        let signal = self.aligned_read.signal();
        let seq_to_signal_map = self.aligned_read
            .query_to_signal()
            .ok_or(SigMapRefineError::RefToSigNotFound)?;

        let seq = self.aligned_read.reference();
        let levels = self.kmer_table.extract_levels(seq)?;

        self.perform_rough_rescaling(
            signal,
            seq_to_signal_map,
            &levels
        )?;
        Ok(())

    }    

    fn perform_rough_rescaling(
        &mut self, 
        signal: &Vec<i16>,
        seq_to_signal_map: &Vec<usize>,
        levels: &Vec<f32>
    ) -> Result<(), SigMapRefineError> {
        match self.settings.rough_rescale_algo() {
            RoughRescaleAlgo::TheilSen { 
                quantiles, 
                clip_bases, 
                use_base_center, 
                max_points } => {
                    (self.scale_dacs_to_norm, self.shift_dacs_to_norm) = rough_rescale_theil_sen(
                        self.scale_dacs_to_norm,
                        self.shift_dacs_to_norm,
                        seq_to_signal_map,
                        levels,
                        signal,
                        quantiles,
                        *clip_bases,
                        *use_base_center,
                        *max_points
                    )?;        
                },

            RoughRescaleAlgo::LeastSquares { 
                quantiles, 
                clip_bases, 
                use_base_center } => {
                    (self.scale_dacs_to_norm, self.shift_dacs_to_norm) = rough_rescale_lstsq(
                        self.scale_dacs_to_norm,
                        self.shift_dacs_to_norm,
                        seq_to_signal_map,
                        levels,
                        signal,
                        quantiles,
                        *clip_bases,
                        *use_base_center
                    )?;        
                },
                
            RoughRescaleAlgo::NoRoughRescaling => {}
        }
        Ok(())
    }
}

/// Calculate the scaling factor and shift to transform the raw signal measurements
/// into normalized measurements.
fn calculate_scaling_shift(
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

// fn refine_signal_mapping(
//     shift: f32, 
//     scale: f32, 
//     seq_to_sig_map: Vec<usize>,
//     sequence: Vec<>,
//     signal: Vec<f16>
// ) {}