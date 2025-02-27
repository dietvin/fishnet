use super::super::error::alignment_errors::reference_to_signal_errors::RefToSignalError;
use rust_htslib::bam::record::Cigar;
use interp::{interp_slice, InterpMode};

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

    let last_el = ref_knots[ref_knots.len()-1];
    let mut interp_vals = Vec::with_capacity((last_el as usize)+1);
    for i in 0..last_el+1 {
        interp_vals.push(i as f64);
    }

    let ref_to_read_knots = interp_slice(
        &ref_knots.iter().map(|el| *el as f64).collect::<Vec<f64>>(), 
        &query_knots.iter().map(|el| *el as f64).collect::<Vec<f64>>(),  
        &interp_vals,
        &InterpMode::FirstLast
    );

    let mut query_to_signal_as_f64 = Vec::new();
    let mut query_to_signal_x_vals = Vec::new();

    for (i, val) in query_to_signal.iter().enumerate() {
        query_to_signal_as_f64.push(*val as f64);
        query_to_signal_x_vals.push(i as f64);
    }

    let ref_to_signal = interp_slice(
        &query_to_signal_x_vals, 
        &query_to_signal_as_f64, 
        &ref_to_read_knots, 
        &InterpMode::FirstLast
    ).iter().map(|el| *el as usize).collect::<Vec<usize>>();

    let ref_to_signal_len = ref_to_signal.len();
    if ref_to_signal_len-1 != reference_len {
        return Err(
            RefToSignalError::DiscordantToSequence(ref_to_signal_len, reference_len)
        );
    }


    Ok(ref_to_signal)
}

/// Determine if the given cigar element is one of Match (M), Equal (=) or Diff (X)
fn is_match_ops(cigar: &Cigar) -> bool {
    if let Cigar::Match(_) | Cigar::Equal(_) | Cigar::Diff(_) = cigar {
        true
    } else {
        false
    }
}

/// Calculate query and reference knots from a given cigar vector. 
/// 
/// # Arguments
/// * `cigar` - Vector containing the cigar elements of the alignment
fn calculate_knots(cigar: &Vec<Cigar>) -> (Vec<u32>, Vec<u32>) {
    let mut current_site_q = 0u32;
    let mut current_site_r = 0u32;
    let mut query_knots = vec![0u32];
    let mut ref_knots = vec![0u32];

    for el in cigar.iter() {
        let cig_len = el.len();
        if consumes_query(el) {
            current_site_q += cig_len;
        }
        if consumes_reference(el) {
            current_site_r += cig_len;
        }
        if is_match_ops(el) {
            query_knots.push(current_site_q - cig_len);
            query_knots.push(current_site_q - 1);

            ref_knots.push(current_site_r - cig_len);
            ref_knots.push(current_site_r - 1);
        }
    }
    
    query_knots.push(current_site_q);
    ref_knots.push(current_site_r);
    
    (query_knots, ref_knots)
}



/// Determine if the given cigar element consumes the reference
/// (i.e. one of Match (M), Deletion (D), RefSkip (N), Equal (=) or Mismatch (X))
fn consumes_reference(cigar: &Cigar) -> bool {
    if let Cigar::Match(_) 
        | Cigar::Del(_) 
        | Cigar::RefSkip(_)
        | Cigar::Equal(_)
        | Cigar::Diff(_) = cigar {
        true
    } else {
        false
    }
}

/// Determine if the given cigar element consumes the query
/// (i.e. one of Match (M), Insertion (I), SoftClip (S), Equal (=) or Mismatch (X))
fn consumes_query(cigar: &Cigar) -> bool {
    if let Cigar::Match(_) 
        | Cigar::Ins(_) 
        | Cigar::SoftClip(_)
        | Cigar::Equal(_)
        | Cigar::Diff(_) = cigar {
        true
    } else {
        false
    }
}
