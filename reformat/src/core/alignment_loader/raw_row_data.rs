use std::sync::Arc;
use pod5_reader_api::dataset::Pod5DatasetThreadSafe;
use uuid::Uuid;

use crate::{
    core::alignment_loader::row::Row, 
    error::core::loader::RawRowDataError
};

/// Contains the raw elements collected from the parquet file.
///
/// Intermediate container for the data that allows for transforming
/// into a [`Row`] within a worker thread during multi-threaded pro-
/// cessing. This struct holds minimally processed data, deferring
/// expensive operations like Pod5 signal loading and normalization
/// until [`into_row`](Self::into_row) is called.
pub(crate) struct RawRowData {
    /// Unique identifier for this sequencing read
    pub read_id: Uuid,
    /// Query/reference-to-signal alignment
    pub alignment: Vec<usize>,
    /// Query/reference shift parameter for normalization
    pub shift: f32,
    /// Query/reference scale parameter for normalization
    pub scale: f32,
    /// Query/reference sequence, if present
    pub sequence: Option<Vec<u8>>,
    /// Reference sequence name this read aligns to (if applicable)
    pub ref_name: Option<String>,
    /// Reference sequence start coordinate this read aligns to 
    /// (if applicable; 1-based coordinate)
    pub ref_start: Option<usize>,
    /// Raw current measurements
    pub signal: Option<Vec<f32>>
}

impl RawRowData {
    /// Constructs a fully processed [`Row`] from raw data.
    ///
    /// This method performs expensive operations including:
    /// - Sequence normalization (uppercase conversion, U->T substitution)
    /// - Pod5 signal loading (if signal not embedded in parquet)
    /// - RNA signal reversal (if applicable)
    /// - Z-score normalization of signal data
    ///
    /// # Arguments
    ///
    /// * `pod5_dataset` - Shared reference to optional Pod5 dataset for signal loading
    /// * `is_rna` - Whether this is RNA data (signals will be reversed)
    /// * `norm_signal` - Whether to apply z-score normalization to signals
    ///
    /// # Errors
    ///
    /// Returns [`RawRowDataError`] if:
    /// - Pod5 dataset is missing when signal data is needed
    /// - Signal standard deviation is zero (cannot normalize)
    /// - Pod5 read access fails
    /// - Statistical calculations fail
    pub fn into_row(
        self,
        pod5_dataset: &Arc<Option<Pod5DatasetThreadSafe>>,
        is_rna: bool
    ) -> Result<Row, RawRowDataError> {
        let sequence = match self.sequence {
            Some(mut bases) => {
                bases.iter_mut().for_each(|c| {
                    *c = match c {
                        b'a'..b'z' => c.to_ascii_uppercase(),
                        _ => *c
                    };
                    if *c == b'U' {
                        *c = b'T'
                    }
                });
                bases
            }
            None => {
                let seq_len = self.alignment.len().saturating_sub(1).max(1);
                vec![b'N'; seq_len]
            }
        };

        let signal = match self.signal {
            Some(signal) => signal,
            None => {
                match pod5_dataset.as_ref() {
                    Some(dataset) => {
                        let mut signal = dataset
                            .get_read(&self.read_id)?
                            .require_signal()?
                            .iter()
                            .map(|&el| (el as f32 - self.shift) / self.scale)
                            .collect::<Vec<f32>>();
                        if is_rna {
                            signal.reverse();
                        }
                        signal
                    }
                    None => return Err(RawRowDataError::Pod5DatasetMissing)
                }
            }
        };

        let row = Row::new(
            self.read_id, 
            self.alignment, 
            sequence, 
            signal, 
            self.ref_name, 
            self.ref_start
        )?;
        Ok(row)
    }
}