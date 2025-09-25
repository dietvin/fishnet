use crate::errors::{InterpolationError, LinspaceError};

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
/// * `Err(InterpolationError)` - An error if interpolation fails
///
/// # Errors
///
/// * `InterpolationError::DifferentLength` - If x_ref and y_ref have different lengths
/// * `InterpolationError::EmptyReference` - If the reference arrays are empty
/// * `InterpolationError::ReferenceUnsorted` - If x_ref is not sorted in ascending order
///
/// # Performance Characteristics
/// 
/// * **Time Complexity**: O(n + m log k) where n = input size, m = query size, k = unique points
/// * **Space Complexity**: O(n) for deduplication, O(m) for results
/// * Uses binary search for interval finding (O(log k) per query)
/// * Optimized O(n) deduplication for sorted inputs
/// 
/// # Examples
///
/// ```ignore
/// let x_ref = vec![0.0, 1.0, 2.0];
/// let y_ref = vec![10.0, 20.0, 30.0];
/// let x_query = vec![0.0, 0.5, 1.0, 1.5, 2.0];
/// let result = interpolate(&x_ref, &y_ref, &x_query).unwrap();
/// assert_eq!(result, vec![10.0, 15.0, 20.0, 25.0, 30.0]);
/// 
/// // Handling duplicates (last occurrence wins)
/// let x_ref = vec![0.0, 1.0, 1.0, 2.0];
/// let y_ref = vec![10.0, 15.0, 20.0, 30.0];
/// let x_query = vec![1.0];
/// let result = interpolate(&x_ref, &y_ref, &x_query).unwrap();
/// assert_eq!(result, vec![20.0]); // Uses the last y-value for x=1.0
/// ```
/// 
pub fn interpolate(x_ref: &[f64], y_ref: &[f64], x_query: &[f64]) -> Result<Vec<f64>, InterpolationError> {
    if x_ref.len() != y_ref.len() {
        return Err(InterpolationError::DifferentLength(x_ref.len(), y_ref.len()));
    } else if x_ref.is_empty() {
        return Err(InterpolationError::EmptyReference);
    } else if !x_ref.windows(2).all(|w| w[0] <= w[1]) {
        // Data must be sorted for linear deduplication and binary search
        return Err(InterpolationError::ReferenceUnsorted);
    } 

    // Handle cases where x_ref has duplicates by only keeping the last occurence
    let mut unique_x = Vec::with_capacity(x_ref.len());
    let mut unique_y = Vec::with_capacity(y_ref.len());

    // Since the x reference coordinates are sorted the deduplication process can
    // run O(n) over the previous O(n^2) approach, where the `position` function
    // was used 
    for (i, (&x_val, &y_val)) in x_ref.iter().zip(y_ref).enumerate() {
        if i == 0 || x_val != unique_x[unique_x.len() - 1] {
            // New unique x-value: add both x and y to the unique arrays
            unique_x.push(x_val);
            unique_y.push(y_val);
        } else {
            // Duplicate x-value: update the corresponding y-value (keep last occurrence)
            let last_idx = unique_y.len() - 1;
            unique_y[last_idx] = y_val;
        }
    }

    let mut result = vec![0.0; x_query.len()];

    // In case only one unique point remains after de-duplication
    // all queries map to the single y-value
    if unique_x.len() == 1 {
        result.fill(unique_y[0]);
        return Ok(result);
    }

    let last_idx = unique_x.len() - 1;
    for (i, &query) in x_query.iter().enumerate() {
        // Handle left extrapolation: queries below the minimum x-value
        if query <= unique_x[0] {
            result[i] = unique_y[0];
            continue;
        } 
        // Handle right extrapolation: queries above the maximum x-value
        else if query >= unique_x[last_idx] {
            result[i] = unique_y[last_idx];
            continue;
        }
        // Handle interpolation: queries within the data range
        else {
            // Find the right interval for interpolation in O(log(n)) via binary search
            let j = unique_x.binary_search_by(|&x| 
                x.partial_cmp(&query).unwrap_or(std::cmp::Ordering::Greater)
            )
                .unwrap_or_else(|idx| idx)
                .saturating_sub(1);

            let dx = unique_x[j + 1] - unique_x[j];
            if dx.abs() < f64::EPSILON {
                // Catches case where the two x-values are very close together
                // (to avoid near-zero division)
                result[i] = unique_y[j];
            } else {
                let slope = (unique_y[j + 1] - unique_y[j]) / dx;
                result[i] = unique_y[j] + slope * (query - unique_x[j]);
            }
        }
    }

    Ok(result)
}


pub fn linspace(start: f64, stop: f64, num: usize) -> Result<Vec<f64>, LinspaceError> {
    if num == 0 {
        return Err(LinspaceError::ZeroElements);
    } else if num == 1 {
        return Ok(vec![start]);
    }

    let mut result = Vec::with_capacity(num);
    // Calculate step size 
    let step = (stop - start) / (num - 1) as f64;

    // Generate the sequence
    for i in 0..num {
        let value = start + step * i as f64;
        result.push(value);
    }
    result[num-1] = stop;

    Ok(result)
}


#[cfg(test)]
mod test {
    use super::{interpolate, linspace};

    #[test]
    fn test_interpolate() {
        let x_ref = vec![0.0, 1.0, 2.0];
        let y_ref = vec![10.0, 20.0, 30.0];
        let x_query = vec![0.0, 0.5, 1.0, 1.5, 2.0];
        let result = interpolate(&x_ref, &y_ref, &x_query).unwrap();
        assert_eq!(result, vec![10.0, 15.0, 20.0, 25.0, 30.0]);
    }

    #[test]
    fn test_linspace_basic() {
        let result = linspace(0.0, 1.0, 5).unwrap();
        assert_eq!(result, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
    }

    #[test]
    fn test_linspace_single_point() {
        let result = linspace(5.0, 10.0, 1).unwrap();
        assert_eq!(result, vec![5.0]);
    }

    #[test]
    fn test_linspace_reverse() {
        let result = linspace(10.0, 0.0, 5).unwrap();
        assert_eq!(result, vec![10.0, 7.5, 5.0, 2.5, 0.0]);
    }
}