use crate::errors::QuantileCalcError;

/// Calculates quantiles for a given data set.
///
/// # Arguments
/// * `data` - The data set to calculate quantiles for
/// * `quantiles` - The quantile values to calculate (values between 0.0 and 1.0)
///
/// # Returns
/// A vector of calculated quantile values, or an error if the calculation fails
pub fn calculate_quantiles(data: &[f32], quantiles: &[f32]) -> Result<Vec<f32>, QuantileCalcError> {
    if data.len() == 0 {
        return Err(QuantileCalcError::EmptyDataVec);
    } else if quantiles.len() == 0 {
        return Err(QuantileCalcError::EmptyQuantVec);
    }

    let mut sorted_data = data.to_vec();
    sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // For each requested quantile value, calculate the corresponding value from sorted data
    quantiles.iter()
        .map(|&q| calculate_single_quantile(&sorted_data, q))
        .collect::<Result<Vec<f32>, QuantileCalcError>>()
}

/// Calculates a single quantile from a sorted vector.
/// 
/// # Arguments
/// * `sorted_data` - The sorted data set to calculate the quantile for
/// * `q` - The quantile calculate (values between 0.0 and 1.0)
/// 
/// # Returns 
/// The calculated quantile value, or an error if the calculation fails
fn calculate_single_quantile(sorted_data: &[f32], q: f32) -> Result<f32, QuantileCalcError> {
    if q > 1.0 || q < 0.0 {
        return Err(QuantileCalcError::InvalidQuant(q));
    }
    
    // Calculate the position in the sorted array (can be fractional)
    let pos = q * (sorted_data.len() - 1) as f32;

    // Get indices for interpolation
    let idx_floor = pos.floor() as usize;
    let idx_ceil = pos.ceil() as usize;

    // If the position is exactly at an index, return that value
    if idx_floor == idx_ceil {
        Ok(sorted_data[idx_floor])
    } 
    // Otherwise, linearly interpolate between the two nearest values
    else {
        let weight_ceil = pos - idx_floor as f32;   // Fractional part of position
        let weight_floor = 1.0 - weight_ceil;       // Complement of fractional part
        // Weighted average of the two values
        Ok(weight_floor * sorted_data[idx_floor] + weight_ceil * sorted_data[idx_ceil])
    }
}


/// Calculate given upper and lower quantile values.
/// 
/// This function calculates two quantiles (an upper and lower) from given data.
/// 
/// # Arguments
/// * `data` - Data from which to calculate the quantile values
/// * `lower_quantiles` - The lower quantile for which to calculate the value
/// * `upper_quantiles` - The upper quantile for which to calculate the value
/// 
/// # Returns
/// A tuple containing the lower and upper quantile values, or an error if the data
/// is empty or the quantile calculation fails
pub fn get_upper_lower_quantiles(
    data: &Vec<f32>, 
    lower_quantiles: f32, 
    upper_quantiles: f32
) -> Result<(f32, f32), QuantileCalcError> {
    if data.len() == 0 {
        return Err(QuantileCalcError::EmptyDataVec);
    }

    let mut sorted_data = data.to_vec();
    sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    Ok((
        calculate_single_quantile(&sorted_data, lower_quantiles)?,
        calculate_single_quantile(&sorted_data, upper_quantiles)?
    ))
}


#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    
    #[test]
    fn test_calculate_quantiles() {
        // Test case 1: Simple vector with exact quantiles
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let quantiles = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let result = calculate_quantiles(&data, &quantiles).unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0]);

        // Test case 2: Vector with interpolated quantiles
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let quantiles = vec![0.0, 0.3, 0.6, 1.0];
        let result = calculate_quantiles(&data, &quantiles).unwrap();
        assert_relative_eq!(result[0], 10.0);
        assert_relative_eq!(result[1], 22.0);
        assert_relative_eq!(result[2], 34.0);
        assert_relative_eq!(result[3], 50.0);

        // Test case 3: Unsorted data
        let data = vec![5.0, 3.0, 1.0, 4.0, 2.0];
        let quantiles = vec![0.0, 0.5, 1.0];
        let result = calculate_quantiles(&data, &quantiles).unwrap();
        assert_eq!(result, vec![1.0, 3.0, 5.0]);
        
        // Test case 4: Empty data
        let data: Vec<f32> = vec![];
        let quantiles = vec![0.5];
        let result = calculate_quantiles(&data, &quantiles);
        assert!(result.is_err());
        
        // Test case 5: Empty quantiles
        let data = vec![1.0, 2.0, 3.0];
        let quantiles: Vec<f32> = vec![];
        let result = calculate_quantiles(&data, &quantiles);
        assert!(result.is_err());
    }
}