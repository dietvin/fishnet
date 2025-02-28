use super::super::error::alignment_errors::reference_to_signal_errors::RefToSignalError;
use rust_htslib::bam::record::Cigar;
use std::io::{self, Write};
use super::helpers::{is_match_ops, calculate_knots, interpolate};

pub fn align_reference_to_signal(
    cigar: &Vec<Cigar>,
    query_to_signal: &Vec<usize>,
    reverse_mapped: bool,
    reference_len: usize
) -> Result<Vec<usize>, RefToSignalError>{
    let mut cigar = cigar.clone();

    if reverse_mapped {
        cigar = cigar.iter().rev().map(|el| *el).collect();
    }

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
        return Err(RefToSignalError::NoMatchOps);
    }
    cigar.truncate(cigar.len()-cutoff_len);

    // Calculate the knots 
    let (query_knots, ref_knots) = calculate_knots(&cigar);

    // The last element corresponds to the number of samples in the signal
    let last_el = ref_knots[ref_knots.len()-1];
    let mut interp_vals = Vec::with_capacity((last_el as usize)+1);
    for i in 0..last_el+1 {
        interp_vals.push(i as f64);
    }

    let ref_to_read_knots = interpolate(
        &ref_knots.iter().map(|el| *el as f64).collect::<Vec<f64>>(), 
        &query_knots.iter().map(|el| *el as f64).collect::<Vec<f64>>(), 
        &interp_vals
    )?;

    let mut query_to_signal_as_f64 = Vec::new();
    let mut query_to_signal_x_vals = Vec::new();

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
            RefToSignalError::DiscordantToSequence(ref_to_signal_len, reference_len)
        );
    }

    Ok(ref_to_signal)
}