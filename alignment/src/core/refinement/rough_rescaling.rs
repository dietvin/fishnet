use crate::error::core::refinement::rescale::RoughRescaleError;

mod prepare;
pub mod least_squares;
pub mod theil_sen;
pub mod skip;


/// Trait defining a coarse (quantile-based) rescaling strategy.
///
/// Rough rescaling provides an initial estimate of normalization parameters
/// using a reduced representation of the data (quantiles instead of per-base
/// statistics). This is used as a fast, robust preconditioning step before
/// more precise rescaling.
///
/// Implementors encapsulate:
/// * A method for extracting representative signal/level summaries
/// * A regression strategy (e.g., least squares or Theil–Sen)
///
/// # Required Methods
///
/// ## `new`
/// Constructs a new algorithm instance with preprocessing configuration.
///
/// * `quantiles` - Quantiles to compute (values in `[0.0, 1.0]`)
/// * `clip_bases` - Number of bases to remove from each end
/// * `use_base_center` - Whether to sample signal at base centers instead of using all points
///
/// ## `rough_rescale`
/// Performs rough rescaling using quantile summaries.
///
/// * `scale`, `shift` - Current normalization parameters
/// * `seq_to_signal_map` - Mapping from base indices to signal indices
/// * `levels` - Expected reference levels
/// * `signal` - Raw signal values
///
/// Returns updated `(scale, shift)` parameters.
///
/// ## `options`
/// Returns the preprocessing configuration.
///
/// # Model
/// Similar to fine rescaling, the method estimates:
///
/// `levels ≈ scale_est * norm_signal + shift_est`
///
/// but uses quantile summaries instead of per-base averages.
///
/// # Notes
/// * Designed for speed and robustness with minimal data.
/// * Accuracy is lower than full rescaling but sufficient for initialization.
pub trait RoughRescaleAlgo: Clone + Send {
    fn new(
        quantiles: Vec<f32>,
        clip_bases: usize,
        use_base_center: bool
    ) -> Self;

    fn rough_rescale(
        &self,
        scale: f32,
        shift: f32,
        seq_to_signal_map: &[usize],
        levels: &[f32],
        signal: &[f32]
    ) -> Result<(f32, f32), RoughRescaleError>;

    fn options(&self) -> &RoughRescaleOptions;
}

/// Configuration for rough rescaling preprocessing.
///
/// # Fields
/// * `quantiles` - Quantiles to compute for both normalized signal and levels
///                 (each value must lie in `[0.0, 1.0]`)
/// * `clip_bases` - Number of bases to exclude from both ends of the sequence
///                  to avoid boundary artifacts
/// * `use_base_center` - If `true`, uses one signal sample per base (center index);
///                       otherwise uses the full signal segment
///
/// # Notes
/// * Quantile-based summarization reduces sensitivity to local noise.
/// * Clipping mitigates edge effects from alignment or segmentation errors.
#[derive(Clone)]
pub struct RoughRescaleOptions {
    pub quantiles: Vec<f32>,
    pub clip_bases: usize,
    pub use_base_center: bool
}