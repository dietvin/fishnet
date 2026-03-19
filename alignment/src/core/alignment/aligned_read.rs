use helper::interpolation::interpolate;
use crate::{
    core::{
        alignment::{
            base_read::BaseRead, 
            helpers::{calculate_knots, is_match_ops},
        },
    },
    error::alignment_errors::{
        QueryAlignedError, RefAlignedError
    }
};

pub(crate) struct QueryAligned<'a> {
    base: BaseRead<'a>,
    pub query_to_signal: Vec<usize>
}

impl<'a> QueryAligned<'a> {
    /// Aligns the query (base-called) sequence to raw signal measurements, producing a 
    /// QueryAligned instance.
    /// 
    /// This function creates a mapping between positions in the base-called sequence and 
    /// positions in the raw signal using the move table generated during base-calling.
    /// The move table indicates when the sequencer detected a new base as it processed
    /// the signal.
    /// 
    /// The resulting alignment is a vector where each index represents a query position
    /// and the value represents the corresponding signal position.
    /// 
    /// # Required data
    /// 
    /// * `move_table` - A slice of boolean values where `true` indicates the sequencer detected
    ///                  a new base at this signal position.
    /// * `stride` - The sampling rate factor - number of signal measurements taken per move table position.
    /// * `signal_len` - The total number of measurements in the raw signal.
    /// * `reverse_signal` - Whether to reverse the mapping direction (set to `true` for direct RNA 
    ///                      data that runs 3'->5' through the pore).
    /// * `query_length` - The length of the base-called sequence in nucleotides.
    /// 
    /// # Arguments
    /// 
    /// * `base_read` - A BaseRead instance containing all needed data for the alignment
    /// 
    /// # Returns
    ///
    /// * `Ok(QueryAligned)` - A QueryAligned instance containing the BaseRead and the QueryAligned instance
    /// * `Err(QueryAlignedError)` - An error if the mapping is inconsistent with query or signal dimensions.
    ///
    /// # Errors
    ///
    /// * `QueryAlignedError::DiscordantToSequence` - If the number of steps in the mapping doesn't match the expected query length.
    /// * `QueryAlignedError::DiscordantToSignal` - If the move table length is inconsistent with the signal length and stride.
    pub(super) fn from_base_read(base_read: BaseRead<'a>) -> Result<Self, QueryAlignedError> {
        let query_length = base_read.query_length();
        let move_table = base_read.move_table();
        let stride = base_read.stride();
        let signal_len = base_read.num_samples_trimmed();

        let mut query_to_signal = Vec::with_capacity(query_length+1);
        for (i, step_forward) in move_table.iter().enumerate() {
            if *step_forward {
                query_to_signal.push(i * stride);
            }
        }
        query_to_signal.push(signal_len);
    
        if base_read.reverse_signal() {
            query_to_signal = query_to_signal
                .iter()
                .rev()
                .map(|el| signal_len - *el)
                .collect();
        }
    
        if query_to_signal.len()-1 != query_length {
            return Err(QueryAlignedError::DiscordantToSequence(
                query_to_signal.len(), query_length
            ));
        } else if move_table.len() != (signal_len/stride) {
            return Err(QueryAlignedError::DiscordantToSignal(
                query_to_signal.len(), signal_len, stride, signal_len/stride)
            );
        }

        Ok(Self { base: base_read, query_to_signal })
    }
}

pub(crate) struct RefAligned<'a> {
    base: BaseRead<'a>,
    pub query_to_signal: Vec<usize>,
    pub ref_to_signal: Vec<usize>
}

impl<'a> RefAligned<'a> {
    /// Aligns a reference sequence to raw signal measurements, producing a 
    /// QueryAligned instance.
    ///
    /// This function creates a mapping between positions in the reference sequence and
    /// positions in the raw signal. It uses the CIGAR string from the alignment
    /// and a pre-computed query-to-signal mapping to perform this transitive alignment.
    ///
    /// The resulting alignment is a vector where each index represents a reference 
    /// position and the value represents the corresponding signal position.
    /// 
    /// # Algorithm
    ///
    /// 1. Process the CIGAR string to create a mapping between reference and query positions
    /// 2. Use the query-to-signal mapping to translate query positions to signal positions
    /// 3. Perform linear interpolation to create a complete reference-to-signal mapping
    ///
    /// # Required data
    ///
    /// * `cigar` - The CIGAR string from the alignment between reference and query sequences
    /// * `query_to_signal` - The pre-computed mapping from query positions to signal positions
    /// * `reverse_mapped` - Whether the alignment was performed in reverse orientation (set 
    ///                      to `true` for direct RNA data that runs 3'->5' through the pore)
    /// * `reference_len` - The length of the reference sequence
    ///
    /// # Arguments
    /// 
    /// * `query_aligned` - QueryAligned instance containing both the BaseRead and the query-
    ///                     to signal alignment
    /// 
    /// # Returns
    ///
    /// * `Ok(Vec<usize>)` - A vector where each index represents a reference position and the value
    ///                      represents the corresponding signal position
    /// * `Err(RefAlignedError)` - An error if the mapping cannot be created
    ///
    /// # Errors
    ///
    /// * `RefAlignedError::NoMatchOps` - If the CIGAR string contains no match operations
    /// * `RefAlignedError::DiscordantToSequence` - If the number of points in the mapping doesn't match the reference length
    /// * `RefAlignedError::LinInterpError` - If linear interpolation fails
    pub(super) fn from_query_aligned(query_aligned: QueryAligned<'a>) -> Result<Self, RefAlignedError> {

        // TODO: Look into merging the two separate interpolation steps into one

        let cigar = query_aligned.base.get_cigar()?;
        let query_to_signal = &query_aligned.query_to_signal;
        let reference_len = query_aligned.base.get_reference_len()?;

        // Non-match operations at the end of the cigar strings must be cut off
        // Determine the number of these operations and remove them from the cigar vector. 
        let mut cutoff_len = 0;
        for (idx, el) in cigar.iter().rev().enumerate() {
            if is_match_ops(el) {
                cutoff_len = idx;
                break;
            }
        }

        if cutoff_len >= cigar.len() {
            return Err(RefAlignedError::NoMatchOps);
        }
        let cigar_slice = &cigar[..cigar.len() - cutoff_len];

        // Calculate the knots 
        let (query_knots, ref_knots) = calculate_knots(cigar_slice);

        // The last element corresponds to the number of samples in the signal
        // Note: calculate_knots now produces f64 vectors, so a conversion is
        // required here. I'll leave the conversion like this for now, but it
        // could potentially give a interp_vals vector of the wrong size
        let last_el = ref_knots[ref_knots.len()-1] as usize;
        let mut interp_vals = Vec::with_capacity((last_el as usize)+1);
        for i in 0..last_el+1 {
            interp_vals.push(i as f64);
        }

        let ref_to_read_knots = interpolate(
            &ref_knots, 
            &query_knots, 
            &interp_vals
        )?;

        let mut query_to_signal_as_f64 = Vec::with_capacity(query_to_signal.len());
        let mut query_to_signal_x_vals = Vec::with_capacity(query_to_signal.len());

        for (i, val) in query_to_signal.iter().enumerate() {
            query_to_signal_as_f64.push(*val as f64);
            query_to_signal_x_vals.push(i as f64);
        }

        let ref_to_signal = interpolate(
            &query_to_signal_x_vals, 
            &query_to_signal_as_f64, 
            &ref_to_read_knots, 
        )?.iter().map(|el| *el as usize).collect::<Vec<usize>>();

        let ref_to_signal_len = ref_to_signal.len();
        if ref_to_signal_len-1 != reference_len {
            return Err(
                RefAlignedError::DiscordantToSequence(ref_to_signal_len, reference_len)
            );
        }

        Ok(Self { 
            base: query_aligned.base,
            query_to_signal: query_aligned.query_to_signal,
            ref_to_signal: ref_to_signal
        })
    }    
}
