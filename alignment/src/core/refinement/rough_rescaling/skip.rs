use crate::{
    core::refinement::rough_rescaling::{RoughRescaleAlgo, RoughRescaleOptions},
    error::core::refinement::rescale::RoughRescaleError
};


/// No-op rough rescaling strategy.
///
/// This implementation bypasses rough rescaling entirely and returns the
/// input parameters unchanged.
///
/// # Behavior
/// * `rough_rescale` returns `(scale, shift)` without modification
/// * `new` ignores all input parameters and initializes default options
///
/// # Use Cases
/// * Disabling rough rescaling in a configurable pipeline
/// * Benchmarking or debugging downstream stages independently
///
/// # Notes
/// * `options` returns a default [`RoughRescaleOptions`] instance with:
///   - empty quantiles
///   - zero clipping
///   - `use_base_center = true`
#[derive(Clone)]
pub struct SkipRoughRescaling {
    options: RoughRescaleOptions
}

impl RoughRescaleAlgo for SkipRoughRescaling {
    fn new(
            _: Vec<f32>,
            _: usize,
            _: bool
        ) -> Self {
        let options = RoughRescaleOptions {
            quantiles: Vec::new(),
            clip_bases: 0,
            use_base_center: true
        };
        Self { options }
    }

    fn rough_rescale(
            &self,
            scale: f32,
            shift: f32,
            _: &[usize],
            _: &[f32],
            _: &[f32]
        ) -> Result<(f32, f32), RoughRescaleError> {
        Ok((scale, shift))
    }

    fn options(&self) -> &RoughRescaleOptions {
        &self.options
    }
}