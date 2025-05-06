use rust_htslib::bam::Record;

use crate::{cli::parse::args_to_input::WhichToAlign, core::refinement::signal_map_refiner::SigMapRefiner, error::output_errors::OutputError};

/// Common trait for all data writers
pub trait DataWriter {
    /// Write a single record using the provided SigMapRefiner
    fn write_record(&mut self, refiner: &mut SigMapRefiner, which_to_align: &WhichToAlign) -> Result<(), OutputError>;

    /// Flush any buffered data to the underlying storage
    fn flush(&mut self) -> Result<(), OutputError>;

    /// Finalize the writer, ensuring all data is written and resources are properly released
    fn finalize(self) -> Result<(), OutputError>;
}