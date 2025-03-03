// ########################################################################################################################
// #                                    Helper functions reference to signal alignment                                    #
// ########################################################################################################################
use rust_htslib::bam::record::Cigar;

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
pub fn is_match_ops(cigar: &Cigar) -> bool {
    if let Cigar::Match(_) | Cigar::Equal(_) | Cigar::Diff(_) = cigar {
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
pub fn calculate_knots(cigar: &Vec<Cigar>) -> (Vec<u32>, Vec<u32>) {
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
pub fn consumes_reference(cigar: &Cigar) -> bool {
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
pub fn consumes_query(cigar: &Cigar) -> bool {
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


use super::super::error::alignment_errors::reference_to_signal_errors::RefToSignalError;

/// Performs linear interpolation similar to NumPy's `interp` function.
///
/// This function interpolates to find the value of new points based on discrete data points.
/// When duplicate x-coordinates exist in the input, only the last occurrence is used,
/// matching NumPy's behavior.
///
/// # Arguments
///
/// * `x_ref` - Reference x coordinates (must be sorted in ascending order)
/// * `y_ref` - Reference y coordinates (values corresponding to x_ref)
/// * `x_query` - The x coordinates at which to evaluate the interpolated values
///
/// # Returns
///
/// * `Ok(Vec<f64>)` - A vector containing the interpolated values corresponding to x_query
/// * `Err(RefToSignalError)` - An error if interpolation fails
///
/// # Errors
///
/// * `RefToSignalError::LinInterpError` - If the input arrays have inconsistent sizes or are empty
///
/// # Examples
///
/// ```
/// let x_ref = vec![0.0, 1.0, 2.0];
/// let y_ref = vec![10.0, 20.0, 30.0];
/// let x_query = vec![0.0, 0.5, 1.0, 1.5, 2.0];
/// let result = interpolate(&x_ref, &y_ref, &x_query).unwrap();
/// assert_eq!(result, vec![10.0, 15.0, 20.0, 25.0, 30.0]);
/// ```
///
/// # Notes
///
/// * For x_query values below the minimum of x_ref, the first y value is returned.
/// * For x_query values above the maximum of x_ref, the last y value is returned.
/// * When x_ref contains duplicate values, only the last occurrence is used for interpolation.
/// * This implementation handles edge cases that can cause NaN values in other interpolation libraries.
pub fn interpolate(x_ref: &[f64], y_ref: &[f64], x_query: &[f64]) -> Result<Vec<f64>, RefToSignalError> {
    if x_ref.len() != y_ref.len() {
        return Err(RefToSignalError::LinInterpError(format!(
            "x_ref and y_ref must have the same length ({} vs {})",
            x_ref.len(), y_ref.len()
        )));
    } else if x_ref.is_empty() {
        return Err(RefToSignalError::LinInterpError(
            "x_ref and y_ref must not be empty".to_string()
        ));
    }

    let mut result = vec![0.0; x_query.len()];

    // Handle cases where x_ref has duplicates by only keeping the last occurence
    let mut unique_x = Vec::with_capacity(x_ref.len());
    let mut unique_y = Vec::with_capacity(y_ref.len());

    // If this x value is already in unique_x, replace its corresponding y value
    for (x_val, y_val) in x_ref.iter().zip(y_ref) {
        if let Some(pos) = unique_x.iter().position(|r| r==x_val) {
            unique_y[pos] = *y_val;
        } else {
            // Otherwise, add new entries
            unique_x.push(*x_val);
            unique_y.push(*y_val);
        }
    }

    // Perform duplication on de-duplicated arrays
    for (i, &query) in x_query.iter().enumerate() {
        // Handle extrapolation or exact match for the lower bound
        if query <= unique_x[0] {
            result[i] = unique_y[0];
            continue;
        }

        // Handle extrapolation or exact match for the upper bound
        if query >= unique_x[unique_x.len() - 1] {
            result[i] = unique_y[unique_y.len() - 1];
            continue;
        }

        // Find the right interval for interpolation
        let mut j = 0;
        while j < unique_x.len() - 1 && query > unique_x[j + 1] {
            j += 1;
        }

        // Linear interpolation within the found interval
        let slope = (unique_y[j + 1] - unique_y[j]) / (unique_x[j + 1] - unique_x[j]);
        result[i] = unique_y[j] + slope * (query - unique_x[j]);
    }

    Ok(result)
}