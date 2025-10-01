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

/// Buffer structure to store different types of processed data until
/// all lines in the buffer are written to file. 
/// 
/// How the lines are stored depends on two factores:
/// 1. The reformat strategy (Base-wise stats / Signal interpolation)
/// 2. The output shape:
///     * `Melted`: Long format of the output data
///     * `Exploded`: Wide format of the output data
///     * `Nested`: Nested format of the output data
pub(crate) enum ArrowBuffer {
    StatsMelted {
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        buffer_region_of_interest: MutableUtf8Array<i32>,
        buffer_base_index: MutablePrimitiveArray<u64>,
        buffer_base: MutableUtf8Array<i32>,
        dynamic_buffer_stats: HashMap<Stats, MutablePrimitiveArray<f64>>,
        stats_in_order: Vec<Stats>
    },
    StatsExploded {
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        buffer_region_of_interest: MutableUtf8Array<i32>,
        dynamic_buffer_bases: Vec<MutableUtf8Array<i32>>,
        // #stats (outer Vec) x #bases (inner Vec) x #reads (Primitive array)
        dynamic_buffer_stats: HashMap<Stats, Vec<MutablePrimitiveArray<f64>>>,
        stats_in_order: Vec<Stats>
    },
    StatsNested {
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        buffer_region_of_interest: MutableUtf8Array<i32>,
        buffer_bases: MutableUtf8Array<i32>,
        dynamic_buffer_stats: HashMap<Stats, MutableListArray<i32, MutablePrimitiveArray<f64>>>,
        stats_in_order: Vec<Stats>
    },
    InterpMelted {
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        buffer_region_of_interest: MutableUtf8Array<i32>,
        buffer_base_index: MutablePrimitiveArray<u64>,
        buffer_base: MutableUtf8Array<i32>,
        dynamic_buffer_signals: Vec<MutablePrimitiveArray<f64>>,
        buffer_dwells: MutablePrimitiveArray<f64>,
    },
    InterpExploded {
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        buffer_region_of_interest: MutableUtf8Array<i32>,
        dynamic_buffer_bases: Vec<MutableUtf8Array<i32>>,
        // #bases (outer Vec) x interpolation target size (inner Vec) x #reads (Primitive array)
        dynamic_buffer_signals: Vec<Vec<MutablePrimitiveArray<f64>>>,
        dynamic_buffer_dwells: Vec<MutablePrimitiveArray<f64>>,
    },
    InterpNested {
        buffer_read_id: MutableUtf8Array<i32>,
        buffer_start_index_on_read: MutablePrimitiveArray<u64>,
        buffer_region_of_interest: MutableUtf8Array<i32>,
        // Bases column contains lists of bases 
        buffer_bases: MutableUtf8Array<i32>,
        // Signals column contains nested lists (#bases (outer Vec) x interpolation target size (inner Vec))
        buffer_signals: MutableListArray<i32, MutableListArray<i32, MutablePrimitiveArray<f64>>>, 
        buffer_dwells: MutableListArray<i32, MutablePrimitiveArray<f64>>
    }
}

impl ArrowBuffer {
    /// Initializes a new arrow buffer
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

    /// Pushes the data generated from a read to the buffer
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
                        read_id_string, 
                        start_index_on_alignment, 
                        matched_region_name, 
                        bases, 
                        stat_collection
                    )?,
                    ArrowBuffer::StatsExploded { .. } => self.push_stats_exploded(
                        read_id_string, 
                        start_index_on_alignment, 
                        matched_region_name, 
                        bases, 
                        stat_collection
                    )?,
                    ArrowBuffer::StatsNested { .. } => self.push_stats_nested(
                        read_id_string, 
                        start_index_on_alignment, 
                        matched_region_name, 
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
                        read_id_string,
                        start_index_on_alignment,
                        matched_region_name,
                        bases,
                        signals,
                        dwells,
                    )?,
                    ArrowBuffer::InterpExploded { .. } => self.push_interp_exploded(
                        read_id_string,
                        start_index_on_alignment,
                        matched_region_name,
                        bases,
                        signals,
                        dwells,
                    )?,
                    ArrowBuffer::InterpNested { .. } => self.push_interp_nested(
                        read_id_string,
                        start_index_on_alignment,
                        matched_region_name,
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

    /// Helper function to push base-wise statistics data to a buffer 
    /// for a melted output shape
    fn push_stats_melted(
        &mut self,
        read_id_string: String, 
        start_index_on_alignment: usize, 
        matched_region_name: String, 
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
                return Err(ArrowBufferError::MeltedStatsMismatch);
            }
        
            for i in 0..bases.len() {
                // Append the same read ID for each row
                buffer_read_id.try_push(Some(read_id_string.clone()))?;
                buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
                buffer_region_of_interest.try_push(Some(matched_region_name.clone()))?;
                buffer_base_index.try_push(Some(i as u64))?;
    
                let base = (*bases
                    .get(i)
                    .ok_or(ArrowBufferError::IndexError(i, bases.len()))?
                    as char).to_string();
                buffer_base.try_push(Some(base))?;
            }
            Ok(())
        } else {
            unreachable!("Already checked before calling the function");
        }
    }

    /// Helper function to push base-wise statistics data to a buffer 
    /// for an exploded output shape
    fn push_stats_exploded(
        &mut self,
        read_id_string: String, 
        start_index_on_alignment: usize, 
        matched_region_name: String, 
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
                return Err(ArrowBufferError::MeltedStatsMismatch);
            }

            buffer_read_id.try_push(Some(read_id_string.clone()))?; // TODO: Check if there is a more efficient way to get around cloning
            buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
            buffer_region_of_interest.try_push(Some(matched_region_name.clone()))?;

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
            unreachable!("Already checked before calling the function");
        }
    }

    /// Helper function to push base-wise statistics data to a buffer 
    /// for a nested output shape
    fn push_stats_nested(
        &mut self,
        read_id_string: String, 
        start_index_on_alignment: usize, 
        matched_region_name: String, 
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
                return Err(ArrowBufferError::MeltedStatsMismatch);
            }

            buffer_read_id.try_push(Some(read_id_string.clone()))?; // TODO: Check if there is a more efficient way to get around cloning
            buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
            buffer_region_of_interest.try_push(Some(matched_region_name.clone()))?;

            let bases = bases.iter().map(|&el| el as char).collect::<String>();
            buffer_bases.try_push(Some(bases))?;
            
            Ok(())
        } else {
            unreachable!("Already checked before calling the function");
        }

    }

    /// Helper function to push interpolated signal data to a buffer 
    /// for a melted output shape
    fn push_interp_melted(
        &mut self,
        read_id_string: String,
        start_index_on_alignment: usize,
        matched_region_name: String,
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
                buffer_read_id.try_push(Some(read_id_string.clone()))?; // TODO: Check if there is a more efficient way to get around cloning
                buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
                buffer_region_of_interest.try_push(Some(matched_region_name.clone()))?;
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
            unreachable!("Already checked before calling the function");
        }

    }

    /// Helper function to push interpolated signal data to a buffer 
    /// for an exploded output shape
    fn push_interp_exploded(
        &mut self,
        read_id_string: String,
        start_index_on_alignment: usize,
        matched_region_name: String,
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

            buffer_read_id.try_push(Some(read_id_string.clone()))?; // TODO: Check if there is a more efficient way to get around cloning
            buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
            buffer_region_of_interest.try_push(Some(matched_region_name.clone()))?;

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
            unreachable!("Already checked before calling the function");
        }
    }

    /// Helper function to push interpolated signal data to a buffer 
    /// for a nested output shape
    fn push_interp_nested(
        &mut self,
        read_id_string: String,
        start_index_on_alignment: usize,
        matched_region_name: String,
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
            buffer_read_id.try_push(Some(read_id_string.clone()))?; // TODO: Check if there is a more efficient way to get around cloning
            buffer_start_index_on_read.try_push(Some(start_index_on_alignment as u64))?;
            buffer_region_of_interest.try_push(Some(matched_region_name.clone()))?;

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
            unreachable!("Already checked before calling the function");
        }
    } 

    /// Transforms the buffered data into an arrow chunk 
    /// to be written to a parquet file
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