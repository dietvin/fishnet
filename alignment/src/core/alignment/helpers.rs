/*!
 * This module contains helper functions used during the intial alignment process.
 */

use noodles::sam::alignment::record::cigar::{op::Kind, Op};

/// Determines if the given CIGAR element represents a sequence match operation.
///
/// # Arguments
/// 
/// * `cigar` - A CIGAR operation to check
///
/// # Returns
///
/// * `true` if the operation is Match (M), Equal (=), or Diff (X)
/// * `false` otherwise
pub fn is_match_ops(cigar: &Op) -> bool {
    let kind = cigar.kind();
    if let Kind::Match | Kind::SequenceMatch | Kind::SequenceMismatch = kind {
        true
    } else {
        false
    }
}

/// Calculates alignment coordinate "knots" from a CIGAR string.
/// 
/// Knots are positions in both the query and reference sequences where
/// match operations begin and end. These knots are used as anchor points
/// for subsequent interpolation to create a complete coordinate mapping.
/// 
/// # Arguments
/// 
/// * `cigar` - Vector containing the CIGAR elements of the alignment
///
/// # Returns
///
/// * `(Vec<u32>, Vec<u32>)` - A tuple containing:
///   * Vector of query sequence positions (knots)
///   * Vector of reference sequence positions (knots)
///
/// # Note
///
/// The returned knots will always include the start (0) and end positions
/// of both sequences.
pub fn calculate_knots(cigar: &Vec<Op>) -> (Vec<u32>, Vec<u32>) {
    let mut current_site_q = 0u32;
    let mut current_site_r = 0u32;
    let mut query_knots = vec![0u32];
    let mut ref_knots = vec![0u32];

    for el in cigar.iter() {
        let cig_len = el.len() as u32;
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



/// Determines if the given CIGAR element consumes the reference sequence.
///
/// # Arguments
/// 
/// * `cigar` - A CIGAR operation to check
///
/// # Returns
///
/// * `true` if the operation consumes the reference (Match, Deletion, RefSkip, Equal, Diff)
/// * `false` otherwise
pub fn consumes_reference(cigar: &Op) -> bool {
    let kind = cigar.kind();
    if let Kind::Match 
        | Kind::Deletion 
        | Kind::Skip
        | Kind::SequenceMatch
        | Kind::SequenceMismatch = kind {
        true
    } else {
        false
    }
}

/// Determines if the given CIGAR element consumes the query sequence.
///
/// # Arguments
/// 
/// * `cigar` - A CIGAR operation to check
///
/// # Returns
///
/// * `true` if the operation consumes the query (Match, Insertion, SoftClip, Equal, Diff)
/// * `false` otherwise
pub fn consumes_query(cigar: &Op) -> bool {
    let kind = cigar.kind();
    if let Kind::Match 
        | Kind::Insertion
        | Kind::SoftClip
        | Kind::SequenceMatch
        | Kind::SequenceMismatch = kind {
        true
    } else {
        false
    }
}