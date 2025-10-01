use std::collections::HashMap;

use arrow2::{
    array::{
        Array, 
        MutableArray, 
        MutableListArray, 
        MutablePrimitiveArray, 
        MutableUtf8Array, 
        TryPush
    }, 
    chunk::Chunk, 
    datatypes::Schema, 
    io::parquet::write::{
        Encoding,
        RowGroupIterator, 
        WriteOptions
    }
};

use crate::{
    core::reformater::reformated::ReformatedData, 
    error::execute::ArrowBufferError, 
    execute::{
        config::{
            OutputShape, 
            ReformatStrategy, 
            Stats
        }, 
        output::output_data::OutputData
    }
};

/// Buffer structure to store processed alignments before writing to Parquet files.
/// 
/// This enum represents different buffer configurations based on:
/// 1. **Reformat strategy**: How the raw data is processed
///    - `ReadWiseStats`: Compute statistical summaries per base position
///    - `Interpolation`: Interpolate signal values to a fixed length per base
/// 2. **Output shape**: How the data is structured in the output
///    - `Melted`: Long format where each base position becomes a separate row
///    - `Exploded`: Wide format where each base position becomes a separate column
///    - `Nested`: Nested format where base positions are stored as arrays within rows
///
/// Each variant maintains buffers for metadata (read IDs, positions, region names) and
/// variant-specific data buffers that match the chosen reformat strategy and output shape.
/// 
/// The following columns are present in all variants:
/// - `read_id`
/// - `start_index_on_read`
/// - `region_of_interest`
pub(crate) enum ArrowBuffer {
    /// Buffer for base-wise statistics in melted (long) format.
    /// Each base position in each read creates one row in the output.
    StatsMelted {
        /// Sequencing read identifiers
        buffer_read_id: MutableUtf8Array<i32>,
        /// Starting position of the region of interest on each read
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        /// Name of the matched reference region/motif for each read
        buffer_region_of_interest: MutableUtf8Array<i32>,
        /// Index of the base position within the region (0, 1, 2, ...)
        buffer_base_index: MutablePrimitiveArray<u64>,
        /// The nucleotide base at each position (A, C, G, T)
        buffer_base: MutableUtf8Array<i32>,
        /// Statistics for each base position, keyed by statistic type
        dynamic_buffer_stats: HashMap<Stats, MutablePrimitiveArray<f64>>,
        /// Ordered list of statistics to maintain consistent column ordering
        /// used when flushing the buffer
        stats_in_order: Vec<Stats>
    },

    /// Buffer for base-wise statistics in exploded (wide) format.
    /// Each read creates one row with columns for each base position.
    StatsExploded {
        /// Sequencing read identifiers
        buffer_read_id: MutableUtf8Array<i32>,
        /// Starting position of the region of interest on each read
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        /// Name of the matched reference region/motif for each read
        buffer_region_of_interest: MutableUtf8Array<i32>,
        /// Bases at each position, with one buffer per base position
        dynamic_buffer_bases: Vec<MutableUtf8Array<i32>>,
        /// Statistics organized as: HashMap<StatType, Vec<BufferPerBasePosition>>
        /// Outer Vec has length equal to number of base positions        
        dynamic_buffer_stats: HashMap<Stats, Vec<MutablePrimitiveArray<f64>>>,
        /// Ordered list of statistics to maintain consistent column ordering
        /// used when flushing the buffer
        stats_in_order: Vec<Stats>
    },

    /// Buffer for base-wise statistics in nested format.
    /// Each read creates one row with array columns containing all base positions.
    StatsNested {
        /// Sequencing read identifiers
        buffer_read_id: MutableUtf8Array<i32>,
        /// Starting position of the region of interest on each read
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        /// Name of the matched reference region/motif for each read
        buffer_region_of_interest: MutableUtf8Array<i32>,
        /// Concatenated bases as a string (e.g., "ACGT")
        buffer_bases: MutableUtf8Array<i32>,
        /// Statistics organized as: HashMap<StatType, Vec<BufferPerBasePosition>>
        /// Outer Vec has length equal to number of base positions        
        dynamic_buffer_stats: HashMap<Stats, MutableListArray<i32, MutablePrimitiveArray<f64>>>,
        /// Ordered list of statistics to maintain consistent column ordering
        /// used when flushing the buffer
        stats_in_order: Vec<Stats>
    },

    /// Buffer for interpolated signal data in melted (long) format.
    /// Each base position in each read creates one row with interpolated signal values.
    InterpMelted {
        /// Sequencing read identifiers        
        buffer_read_id: MutableUtf8Array<i32>,
        /// Starting position of the region of interest on each read        
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        /// Name of the matched reference region/motif for each read        
        buffer_region_of_interest: MutableUtf8Array<i32>,
        /// Index of the base position within the region (0, 1, 2, ...)
        buffer_base_index: MutablePrimitiveArray<u64>,
        /// The nucleotide base at each position (A, C, G, T)
        buffer_base: MutableUtf8Array<i32>,
        /// Interpolated signal values, with one buffer per interpolation position
        /// Outer Vec has length equal to interpolation target length
        dynamic_buffer_signals: Vec<MutablePrimitiveArray<f64>>,
        /// Dwell time for each base
        buffer_dwells: MutablePrimitiveArray<f64>,
    },

    /// Buffer for interpolated signal data in exploded (wide) format.
    /// Each read creates one row with columns for each base and interpolation position.
    InterpExploded {
        /// Sequencing read identifiers
        buffer_read_id: MutableUtf8Array<i32>,
        /// Starting position of the region of interest on each read
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        /// Name of the matched reference region/motif for each read
        buffer_region_of_interest: MutableUtf8Array<i32>,
        /// Bases at each position, with one buffer per base position
        dynamic_buffer_bases: Vec<MutableUtf8Array<i32>>,
        /// Interpolated signals organized as: Vec<BasePosition, Vec<InterpolationPosition>>
        /// Outer Vec has length equal to number of bases
        /// Inner Vec has length equal to interpolation target length
        dynamic_buffer_signals: Vec<Vec<MutablePrimitiveArray<f64>>>,
        /// Dwell times with one buffer per base position
        dynamic_buffer_dwells: Vec<MutablePrimitiveArray<f64>>,
    },

    /// Buffer for interpolated signal data in nested format.
    /// Each read creates one row with nested array columns.
    InterpNested {
        /// Sequencing read identifiers
        buffer_read_id: MutableUtf8Array<i32>,
        /// Starting position of the region of interest on each read
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        /// Name of the matched reference region/motif for each read
        buffer_region_of_interest: MutableUtf8Array<i32>,
        /// Concatenated bases as a string (e.g., "ACGT")
        buffer_bases: MutableUtf8Array<i32>,
        /// Nested list of signals: outer list for bases, inner list for interpolation positions
        /// Structure: List<List<Float64>> where outer list has one element per base
        buffer_signals: MutableListArray<i32, MutableListArray<i32, MutablePrimitiveArray<f64>>>, 
        /// Dwell times stored as a list, one array per read containing dwells for all bases
        buffer_dwells: MutableListArray<i32, MutablePrimitiveArray<f64>>
    }
}

impl ArrowBuffer {
    /// Creates a new arrow buffer with the appropriate variant and pre-allocated capacity.
    ///
    /// # Arguments
    /// * `reformat_strategy` - The data processing strategy (stats or interpolation)
    /// * `output_shape` - The desired output format (melted, exploded, or nested)
    /// * `buffer_size` - Initial capacity for the buffers (number of records to pre-allocate)
    /// * `uniform_roi_length` - Required for exploded format: the uniform length of all regions of interest
    ///
    /// # Returns
    /// An initialized `ArrowBuffer` variant matching the strategy and shape
    ///
    /// # Panics
    /// Panics if `output_shape` is `Exploded` but `uniform_roi_length` is `None`
    /// This should never happen, since it gets ensured before processing start
    /// that all regions of interest have the same length.
    pub(crate) fn new(
        reformat_strategy: &ReformatStrategy, 
        output_shape: &OutputShape,
        buffer_size: usize,
        uniform_roi_length: Option<usize>
    ) -> Self {
        match (reformat_strategy, output_shape) {
            (ReformatStrategy::ReadWiseStats { stats }, OutputShape::Melted) => {
                let mut dynamic_buffer_stats = HashMap::with_capacity(stats.len());
                for stat in stats {
                    dynamic_buffer_stats.insert(stat.clone(), MutablePrimitiveArray::<f64>::with_capacity(buffer_size));
                }
                Self::StatsMelted { 
                    buffer_read_id: MutableUtf8Array::<i32>::with_capacity(buffer_size), 
                    buffer_start_index_on_read: MutablePrimitiveArray::<u64>::with_capacity(buffer_size), 
                    buffer_region_of_interest: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                    buffer_base_index: MutablePrimitiveArray::<u64>::with_capacity(buffer_size), 
                    buffer_base: MutableUtf8Array::<i32>::with_capacity(buffer_size), 
                    dynamic_buffer_stats,
                    stats_in_order: stats.clone()
                }
            }
            (ReformatStrategy::ReadWiseStats { stats }, OutputShape::Exploded) => {
                if let Some(num_bases) = uniform_roi_length {
                    let mut dynamic_buffer_stats = HashMap::with_capacity(stats.len());
                    for stat in stats {
                        dynamic_buffer_stats.insert(
                            stat.clone(), 
                            vec![MutablePrimitiveArray::<f64>::with_capacity(buffer_size); num_bases]
                        );
                    }
                    Self::StatsExploded { 
                        buffer_read_id: MutableUtf8Array::<i32>::with_capacity(buffer_size), 
                        buffer_start_index_on_read: MutablePrimitiveArray::<u64>::with_capacity(buffer_size),
                        buffer_region_of_interest: MutableUtf8Array::<i32>::with_capacity(buffer_size), 
                        dynamic_buffer_bases: vec![MutableUtf8Array::<i32>::with_capacity(buffer_size); num_bases], 
                        dynamic_buffer_stats: dynamic_buffer_stats,
                        stats_in_order: stats.clone()
                    }
                } else {
                    unreachable!("It's checked before that all regions of interest have the same length when output shape is Exploded")
                }
            }
            (ReformatStrategy::ReadWiseStats { stats }, OutputShape::Nested) => {
                let mut dynamic_buffer_stats = HashMap::with_capacity(stats.len());
                for stat in stats {
                    dynamic_buffer_stats.insert(
                        stat.clone(),
                        MutableListArray::<i32, MutablePrimitiveArray<f64>>::with_capacity(buffer_size)
                    );
                }
                Self::StatsNested { 
                    buffer_read_id: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                    buffer_start_index_on_read: MutablePrimitiveArray::<u64>::with_capacity(buffer_size),
                    buffer_region_of_interest: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                    buffer_bases: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                    dynamic_buffer_stats: dynamic_buffer_stats,
                    stats_in_order: stats.clone()
                }
            }
            (ReformatStrategy::Interpolation { target_len }, OutputShape::Melted) => Self::InterpMelted { 
                buffer_read_id: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                buffer_start_index_on_read: MutablePrimitiveArray::<u64>::with_capacity(buffer_size),
                buffer_region_of_interest: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                buffer_base_index: MutablePrimitiveArray::<u64>::with_capacity(buffer_size),
                buffer_base: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                dynamic_buffer_signals: vec![MutablePrimitiveArray::<f64>::with_capacity(buffer_size); *target_len],
                buffer_dwells: MutablePrimitiveArray::<f64>::with_capacity(buffer_size)
            },
            (ReformatStrategy::Interpolation { target_len }, OutputShape::Exploded) => { 
                if let Some(num_bases) = uniform_roi_length {
                    Self::InterpExploded{
                        buffer_read_id: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                        buffer_start_index_on_read: MutablePrimitiveArray::<u64>::with_capacity(buffer_size),
                        buffer_region_of_interest: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                        dynamic_buffer_bases: vec![MutableUtf8Array::<i32>::with_capacity(buffer_size); num_bases],
                        dynamic_buffer_signals: vec![vec![MutablePrimitiveArray::<f64>::with_capacity(buffer_size); *target_len]; num_bases],
                        dynamic_buffer_dwells: vec![MutablePrimitiveArray::<f64>::with_capacity(buffer_size); num_bases]
                    }
                } else {
                    unreachable!("It's checked before that all regions of interest have the same length when output shape is Exploded")
                }
            },
            (ReformatStrategy::Interpolation { .. }, OutputShape::Nested) => Self::InterpNested { 
                buffer_read_id: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                buffer_start_index_on_read: MutablePrimitiveArray::<u64>::with_capacity(buffer_size),
                buffer_region_of_interest: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                buffer_bases: MutableUtf8Array::<i32>::with_capacity(buffer_size),
                buffer_signals: MutableListArray::<i32, MutableListArray<i32, MutablePrimitiveArray<f64>>>::with_capacity(buffer_size),
                buffer_dwells: MutableListArray::<i32, MutablePrimitiveArray<f64>>::with_capacity(buffer_size)
            }
        }
    }

    /// Adds processed data from a single read to the buffer.
    ///
    /// This method dispatches to the appropriate helper method based on the buffer variant
    /// and the type of reformatted data (statistics vs interpolation).
    ///
    /// # Arguments
    /// * `output_data` - Processed data for one read, including metadata and reformatted values
    ///
    /// # Returns
    /// * `Ok(())` if data was successfully added to the buffer
    /// * `Err(ArrowBufferError)` if there was a type mismatch, index error, or other issue
    pub(crate) fn push_data(
        &mut self,
        output_data: OutputData
    ) -> Result<(), ArrowBufferError> {
        let (
            read_id,
            start_index_on_alignment,
            matched_region_name,
            reformated_data
        ) = output_data.into_inner();

        let read_id_string = read_id.to_string();

        match reformated_data {
            ReformatedData::Stats(data_stats) => {
                let (bases, stat_collection) = data_stats.into_inner()?;

                match self {
                    ArrowBuffer::StatsMelted { .. } => self.push_stats_melted(
                        &read_id_string, 
                        start_index_on_alignment, 
                        &matched_region_name, 
                        bases, 
                        stat_collection
                    )?,
                    ArrowBuffer::StatsExploded { .. } => self.push_stats_exploded(
                        &read_id_string, 
                        start_index_on_alignment, 
                        &matched_region_name, 
                        bases, 
                        stat_collection
                    )?,
                    ArrowBuffer::StatsNested { .. } => self.push_stats_nested(
                        &read_id_string, 
                        start_index_on_alignment, 
                        &matched_region_name, 
                        bases, 
                        stat_collection
                    )?,
                    _ => return Err(ArrowBufferError::UnexpectedBufferTypeWithStats)
                }
            }
            ReformatedData::Interp(data_interp) => {
                let (bases, signals, dwells) = data_interp.into_inner()?;
                match self {
                    ArrowBuffer::InterpMelted { .. } => self.push_interp_melted(
                        &read_id_string,
                        start_index_on_alignment,
                        &matched_region_name,
                        bases,
                        signals,
                        dwells,
                    )?,
                    ArrowBuffer::InterpExploded { .. } => self.push_interp_exploded(
                        &read_id_string,
                        start_index_on_alignment,
                        &matched_region_name,
                        bases,
                        signals,
                        dwells,
                    )?,
                    ArrowBuffer::InterpNested { .. } => self.push_interp_nested(
                        &read_id_string,
                        start_index_on_alignment,
                        &matched_region_name,
                        bases,
                        signals,
                        dwells,
                    )?,
                    _ => return Err(ArrowBufferError::UnexpectedBufferTypeWithInterp)
                }
            }
        }
        Ok(())
    }

    /// Helper function to add base-wise statistics data to a melted format buffer.
    ///
    /// Creates one row per base position, repeating read metadata for each base.
    ///
    /// # Arguments
    /// * `read_id` - Unique identifier for the sequencing read
    /// * `start_index_on_alignment` - Starting position of the ROI on the read
    /// * `matched_region_name` - Name of the matched reference region/motif
    /// * `bases` - Vector of nucleotide bases (as u8 ASCII values)
    /// * `stat_collection` - Map of statistic types to their values for each base
    ///
    /// # Returns
    /// * `Ok(())` if data was successfully buffered
    /// * `Err(ArrowBufferError)` if statistics don't match buffer configuration or index errors occur
    fn push_stats_melted(
        &mut self,
        read_id: &str, 
        start_index_on_alignment: usize, 
        matched_region_name: &str, 
        bases: Vec<u8>,
        stat_collection: HashMap<Stats, Vec<f64>>,
    ) -> Result<(), ArrowBufferError> {
        if let ArrowBuffer::StatsMelted { 
            buffer_read_id, 
            buffer_start_index_on_read, 
            buffer_region_of_interest, 
            buffer_base_index, 
            buffer_base, 
            dynamic_buffer_stats ,
            ..
        } = self {
            // Check if the stats in the Buffer and the reformated data match
            if stat_collection.len() == dynamic_buffer_stats.len() && 
                stat_collection.keys().all(|k|dynamic_buffer_stats.contains_key(k))
            {
                for (stat, values) in stat_collection {
                    let stats_buffer = dynamic_buffer_stats
                        .get_mut(&stat)
                        .ok_or(ArrowBufferError::KeyError(stat))?;
                    stats_buffer.extend_from_slice(&values);
                }
            } else {
                return Err(ArrowBufferError::StatsMismatch);
            }
        
            for i in 0..bases.len() {
                // Append the same read ID for each row
                buffer_read_id.try_push(Some(read_id))?;
                buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
                buffer_region_of_interest.try_push(Some(matched_region_name))?;
                buffer_base_index.try_push(Some(i as u64))?;
    
                let base = (*bases
                    .get(i)
                    .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                    as char).to_string();
                buffer_base.try_push(Some(base))?;
            }
            Ok(())
        } else {
            unreachable!("Already checked in ArrowBuffer::push_data");
        }
    }

    /// Helper function to add base-wise statistics data to an exploded format buffer.
    ///
    /// Creates one row per read with separate columns for each base position.
    ///
    /// # Arguments
    /// * `read_id` - Unique identifier for the sequencing read
    /// * `start_index_on_alignment` - Starting position of the ROI on the read
    /// * `matched_region_name` - Name of the matched reference region/motif
    /// * `bases` - Vector of nucleotide bases (as u8 ASCII values)
    /// * `stat_collection` - Map of statistic types to their values for each base
    ///
    /// # Returns
    /// * `Ok(())` if data was successfully buffered
    /// * `Err(ArrowBufferError)` if statistics don't match buffer configuration or index errors occur
    fn push_stats_exploded(
        &mut self,
        read_id: &str, 
        start_index_on_alignment: usize, 
        matched_region_name: &str, 
        bases: Vec<u8>,
        stat_collection: HashMap<Stats, Vec<f64>>,
    ) -> Result<(), ArrowBufferError> {
        if let ArrowBuffer::StatsExploded { 
            buffer_read_id, 
            buffer_start_index_on_read, 
            buffer_region_of_interest, 
            dynamic_buffer_bases, 
            dynamic_buffer_stats,
            ..
        } = self {
            if !(stat_collection.len() == dynamic_buffer_stats.len() && 
                 stat_collection.keys().all(|k|dynamic_buffer_stats.contains_key(k))
            ) {
                return Err(ArrowBufferError::StatsMismatch);
            }

            buffer_read_id.try_push(Some(read_id.to_string()))?; // TODO: Check if there is a more efficient way to get around cloning
            buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
            buffer_region_of_interest.try_push(Some(matched_region_name))?;

            for i in 0..bases.len() {    
                dynamic_buffer_bases
                    .get_mut(i)
                    .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                    .try_push(Some((bases[i] as char).to_string()))?;

                for (stat, values) in &stat_collection {
                    dynamic_buffer_stats
                        .get_mut(stat).unwrap()
                        .get_mut(i)
                        .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                        .try_push(Some(*values
                            .get(i)
                            .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                        ))?;
                }               
            }

            Ok(())
        } else {
            unreachable!("Already checked in ArrowBuffer::push_data");
        }
    }

    /// Helper function to add base-wise statistics data to a nested format buffer.
    ///
    /// Creates one row per read with array columns containing values for all bases.
    ///
    /// # Arguments
    /// * `read_id` - Unique identifier for the sequencing read
    /// * `start_index_on_alignment` - Starting position of the ROI on the read
    /// * `matched_region_name` - Name of the matched reference region/motif
    /// * `bases` - Vector of nucleotide bases (as u8 ASCII values)
    /// * `stat_collection` - Map of statistic types to their values for each base
    ///
    /// # Returns
    /// * `Ok(())` if data was successfully buffered
    /// * `Err(ArrowBufferError)` if statistics don't match buffer configuration
    fn push_stats_nested(
        &mut self,
        read_id: &str, 
        start_index_on_alignment: usize, 
        matched_region_name: &str, 
        bases: Vec<u8>,
        stat_collection: HashMap<Stats, Vec<f64>>,
    ) -> Result<(), ArrowBufferError> {
        if let ArrowBuffer::StatsNested { 
            buffer_read_id, 
            buffer_start_index_on_read, 
            buffer_region_of_interest, 
            buffer_bases, 
            dynamic_buffer_stats,
            ..
        } = self {
            if stat_collection.len() == dynamic_buffer_stats.len() && 
                stat_collection.keys().all(|k|dynamic_buffer_stats.contains_key(k))
            {
                for (stat, values) in stat_collection {
                    let values = values.iter()
                        .map(|&v| Some(v))
                        .collect::<Vec<Option<f64>>>();
                    let stats_buffer = dynamic_buffer_stats
                        .get_mut(&stat)
                        .ok_or(ArrowBufferError::KeyError(stat))?;
                    stats_buffer.try_push(Some(values))?;
                }
            } else {
                return Err(ArrowBufferError::StatsMismatch);
            }

            buffer_read_id.try_push(Some(read_id))?;
            buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
            buffer_region_of_interest.try_push(Some(matched_region_name))?;

            let bases = bases.iter().map(|&el| el as char).collect::<String>();
            buffer_bases.try_push(Some(bases))?;
            
            Ok(())
        } else {
            unreachable!("Already checked in ArrowBuffer::push_data");
        }

    }

    /// Helper function to add interpolated signal data to a melted format buffer.
    ///
    /// Creates one row per base position with interpolated signal values and dwell time.
    ///
    /// # Arguments
    /// * `read_id` - Unique identifier for the sequencing read
    /// * `start_index_on_alignment` - Starting position of the ROI on the read
    /// * `matched_region_name` - Name of the matched reference region/motif
    /// * `bases` - Vector of nucleotide bases (as u8 ASCII values)
    /// * `signals` - Interpolated signal values: Vec<BasePosition, Vec<InterpolatedValue>>
    /// * `dwells` - Dwell time for each base position
    ///
    /// # Returns
    /// * `Ok(())` if data was successfully buffered
    /// * `Err(ArrowBufferError)` if index errors occur
    fn push_interp_melted(
        &mut self,
        read_id: &str,
        start_index_on_alignment: usize,
        matched_region_name: &str,
        bases: Vec<u8>,
        signals: Vec<Vec<f64>>,
        dwells: Vec<f64>,
    ) -> Result<(), ArrowBufferError> {
        if let ArrowBuffer::InterpMelted { 
            buffer_read_id, 
            buffer_start_index_on_read, 
            buffer_region_of_interest, 
            buffer_base_index, 
            buffer_base, 
            dynamic_buffer_signals, 
            buffer_dwells 
        } = self {
            for i in 0..bases.len() {
                buffer_read_id.try_push(Some(read_id))?;
                buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
                buffer_region_of_interest.try_push(Some(matched_region_name))?;
                buffer_base_index.try_push(Some(i as u64))?;

                let base = (*bases
                    .get(i)
                    .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                    as char).to_string();
                buffer_base.try_push(Some(base))?;

                // Get the signal for base i
                let signal = signals
                    .get(i)
                    .ok_or(ArrowBufferError::IndexError(i, signals.len()))?;

                for (signal_idx, signal_value) in signal.iter().enumerate() {
                    dynamic_buffer_signals
                        .get_mut(signal_idx)
                        .ok_or(ArrowBufferError::IndexError(i, signal.len()))?
                        .try_push(Some(*signal_value))?;
                }

                buffer_dwells.try_push(Some(*dwells
                    .get(i)
                    .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                ))?;
            }

            Ok(())
        } else {
            unreachable!("Already checked in ArrowBuffer::push_data");
        }

    }

    /// Helper function to add interpolated signal data to an exploded format buffer.
    ///
    /// Creates one row per read with separate columns for each base and interpolation position.
    ///
    /// # Arguments
    /// * `read_id` - Unique identifier for the sequencing read
    /// * `start_index_on_alignment` - Starting position of the ROI on the read
    /// * `matched_region_name` - Name of the matched reference region/motif
    /// * `bases` - Vector of nucleotide bases (as u8 ASCII values)
    /// * `signals` - Interpolated signal values: Vec<BasePosition, Vec<InterpolatedValue>>
    /// * `dwells` - Dwell time for each base position
    ///
    /// # Returns
    /// * `Ok(())` if data was successfully buffered
    /// * `Err(ArrowBufferError)` if index errors occur
    fn push_interp_exploded(
        &mut self,
        read_id: &str,
        start_index_on_alignment: usize,
        matched_region_name: &str,
        bases: Vec<u8>,
        signals: Vec<Vec<f64>>,
        dwells: Vec<f64>,
    ) -> Result<(), ArrowBufferError> {
        if let ArrowBuffer::InterpExploded { 
            buffer_read_id, 
            buffer_start_index_on_read, 
            buffer_region_of_interest, 
            dynamic_buffer_bases, 
            dynamic_buffer_signals, 
            dynamic_buffer_dwells 
        } = self {

            buffer_read_id.try_push(Some(read_id))?; // TODO: Check if there is a more efficient way to get around cloning
            buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
            buffer_region_of_interest.try_push(Some(matched_region_name))?;

            for i in 0..bases.len() {    
                dynamic_buffer_bases
                    .get_mut(i)
                    .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                    .try_push(Some((bases[i] as char).to_string()))?;

                // Get the signal for base i
                let signal = signals
                    .get(i)
                    .ok_or(ArrowBufferError::IndexError(i, signals.len()))?;

                for (signal_idx, signal_value) in signal.iter().enumerate() {
                    dynamic_buffer_signals
                        .get_mut(i)
                        .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                        .get_mut(signal_idx)
                        .ok_or(ArrowBufferError::IndexError(i, signal.len()))?
                        .try_push(Some(*signal_value))?;
                }

                dynamic_buffer_dwells
                    .get_mut(i)
                    .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                    .try_push(Some(*dwells
                        .get(i)
                        .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                    ))?;
            }

            Ok(())
        } else {
            unreachable!("Already checked in ArrowBuffer::push_data");
        }
    }

    /// Helper function to add interpolated signal data to a nested format buffer.
    ///
    /// Creates one row per read with nested array columns for signals and dwells.
    ///
    /// # Arguments
    /// * `read_id` - Unique identifier for the sequencing read
    /// * `start_index_on_alignment` - Starting position of the ROI on the read
    /// * `matched_region_name` - Name of the matched reference region/motif
    /// * `bases` - Vector of nucleotide bases (as u8 ASCII values)
    /// * `signals` - Interpolated signal values: Vec<BasePosition, Vec<InterpolatedValue>>
    /// * `dwells` - Dwell time for each base position
    ///
    /// # Returns
    /// * `Ok(())` if data was successfully buffered
    /// * `Err(ArrowBufferError)` if index errors occur
    fn push_interp_nested(
        &mut self,
        read_id: &str,
        start_index_on_alignment: usize,
        matched_region_name: &str,
        bases: Vec<u8>,
        signals: Vec<Vec<f64>>,
        dwells: Vec<f64>,
    ) -> Result<(), ArrowBufferError> {
        if let ArrowBuffer::InterpNested { 
            buffer_read_id, 
            buffer_start_index_on_read, 
            buffer_region_of_interest, 
            buffer_bases, 
            buffer_signals, 
            buffer_dwells 
        } = self {
            buffer_read_id.try_push(Some(read_id))?; // TODO: Check if there is a more efficient way to get around cloning
            buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
            buffer_region_of_interest.try_push(Some(matched_region_name))?;

            let bases = bases.iter().map(|&el| el as char).collect::<String>();
            buffer_bases.try_push(Some(bases))?;

            let signals = signals.iter()
                .map(|signal_for_base| Some(signal_for_base
                    .iter()
                    .map(|&el| Some(el))
                    .collect::<Vec<Option<f64>>>())
                )
                .collect::<Vec<Option<Vec<Option<f64>>>>>();
            buffer_signals.try_push(Some(signals))?;

            let dwells = dwells.iter().map(|&el| Some(el)).collect::<Vec<Option<f64>>>();
            buffer_dwells.try_push(Some(dwells))?;

            Ok(())
        } else {
            unreachable!("Already checked in ArrowBuffer::push_data");
        }
    } 

    /// Converts the buffered data into an Arrow row group iterator for writing to Parquet.
    ///
    /// This method finalizes all mutable arrays into immutable Arrow arrays, assembles them
    /// into a chunk matching the provided schema, and creates a row group iterator.
    ///
    /// # Arguments
    /// * `schema` - Arrow schema defining the structure and types of the output columns
    /// * `encodings` - Encoding specifications for each column in the Parquet file
    /// * `options` - Write options including compression and version settings
    ///
    /// # Returns
    /// A row group iterator ready to be written to a Parquet file, or an error if:
    /// * Column count doesn't match schema
    /// * Statistics are missing from dynamic buffers
    /// * Arrow conversion fails
    pub(crate) fn buffer_to_rowgroupiter(
        &mut self,
        schema: &Schema,
        encodings: &Vec<Vec<Encoding>>,
        options: &WriteOptions
    ) -> Result<RowGroupIterator<Box<dyn Array>, std::iter::Once<Result<Chunk<Box<dyn Array>>, arrow2::error::Error>>>, ArrowBufferError> {
        let mut columns: Vec<Box<dyn Array>> = vec![];

        match self {
            ArrowBuffer::StatsMelted { 
                buffer_read_id, 
                buffer_start_index_on_read, 
                buffer_region_of_interest, 
                buffer_base_index, 
                buffer_base, 
                dynamic_buffer_stats,
                stats_in_order
            } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_start_index_on_read.as_box());
                columns.push(buffer_region_of_interest.as_box());
                columns.push(buffer_base_index.as_box());
                columns.push(buffer_base.as_box());

                for stat in stats_in_order {
                    let buffer = dynamic_buffer_stats
                        .get_mut(&stat)
                        .ok_or(ArrowBufferError::InvalidStat(stat.clone()))?;
                    columns.push(buffer.as_box());
                }
            }

            ArrowBuffer::StatsExploded { 
                buffer_read_id, 
                buffer_start_index_on_read, 
                buffer_region_of_interest, 
                dynamic_buffer_bases, 
                dynamic_buffer_stats,
                stats_in_order
            } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_start_index_on_read.as_box());
                columns.push(buffer_region_of_interest.as_box());

                // Bases buffers are stored in a vector, so they are ordered correctly
                for buffer in dynamic_buffer_bases {
                    columns.push(buffer.as_box());
                }

                for stat in stats_in_order {
                    for buffer in dynamic_buffer_stats
                        .get_mut(&stat)
                        .ok_or(ArrowBufferError::InvalidStat(stat.clone()))? 
                    {
                        columns.push(buffer.as_box());
                    }
                }
            }

            ArrowBuffer::StatsNested { 
                buffer_read_id, 
                buffer_start_index_on_read, 
                buffer_region_of_interest, 
                buffer_bases, 
                dynamic_buffer_stats,
                stats_in_order
            } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_start_index_on_read.as_box());
                columns.push(buffer_region_of_interest.as_box());
                columns.push(buffer_bases.as_box());

                for stat in stats_in_order {
                    let buffer = dynamic_buffer_stats
                        .get_mut(&stat)
                        .ok_or(ArrowBufferError::InvalidStat(stat.clone()))?;
                    columns.push(buffer.as_box());
                }
            }

            ArrowBuffer::InterpMelted { 
                buffer_read_id, 
                buffer_start_index_on_read, 
                buffer_region_of_interest, 
                buffer_base_index, 
                buffer_base, 
                dynamic_buffer_signals, 
                buffer_dwells 
            } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_start_index_on_read.as_box());
                columns.push(buffer_region_of_interest.as_box());
                columns.push(buffer_base_index.as_box());
                columns.push(buffer_base.as_box());

                for buffer in dynamic_buffer_signals {
                    columns.push(buffer.as_box());
                }

                columns.push(buffer_dwells.as_box());
            }

            ArrowBuffer::InterpExploded { 
                buffer_read_id, 
                buffer_start_index_on_read, 
                buffer_region_of_interest, 
                dynamic_buffer_bases, 
                dynamic_buffer_signals, 
                dynamic_buffer_dwells 
            } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_start_index_on_read.as_box());
                columns.push(buffer_region_of_interest.as_box());

                for buffer in dynamic_buffer_bases {
                    columns.push(buffer.as_box());
                }

                for base_buffer in dynamic_buffer_signals {
                    for buffer in base_buffer {
                        columns.push(buffer.as_box());
                    }
                }

                for buffer in dynamic_buffer_dwells {
                    columns.push(buffer.as_box());
                }
            }

            ArrowBuffer::InterpNested { 
                buffer_read_id, 
                buffer_start_index_on_read, 
                buffer_region_of_interest, 
                buffer_bases, 
                buffer_signals, 
                buffer_dwells
            } => {
                columns.push(buffer_read_id.as_box());
                columns.push(buffer_start_index_on_read.as_box());
                columns.push(buffer_region_of_interest.as_box());
                columns.push(buffer_bases.as_box());
                columns.push(buffer_signals.as_box());
                columns.push(buffer_dwells.as_box())
            }
        }

        let chunk = Chunk::try_new(columns)?;
        let row_group_iterator = RowGroupIterator::try_new(
            std::iter::once(Ok(chunk)), 
            schema, 
            options.clone(), 
            encodings.clone()
        )?;
        
        Ok(row_group_iterator)
    }
}