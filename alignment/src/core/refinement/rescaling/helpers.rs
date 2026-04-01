use rand::{rng, seq::IteratorRandom};

use crate::error::core::refinement::rescale::TheilSenError;



/// Calculates the median of a **sorted** vector of floats.
///
/// # Arguments
/// * `vec` - A sorted vector of f32 values
///
/// # Returns
/// The median value, or an error if the calculation fails
pub(super) fn median(vec: &[f32]) -> Result<f32, TheilSenError> {
    let len = vec.len();
    if len == 0 {
        return Err(TheilSenError::MedianCalcEmptyVec);
    }
    
    if len % 2 == 0 {
        Ok((vec[len / 2 - 1] + vec[len / 2]) / 2.0)
    } else {
        Ok(vec[len / 2])
    }
}

/// Returns a random subset of indices from a vector of a given size.
///
/// # Arguments
/// * `vec_len` - The length of the vector to sample from
/// * `downsampled_len` - The number of unique indices to return
///
/// # Returns
/// A vector of unique random indices
pub(super) fn random_subset(vec_len: usize, downsampled_len: usize) -> Vec<usize> {
    (0..vec_len).choose_multiple(&mut rng(), downsampled_len)
}


#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_least_squares() {
    //     // Test case 1: Perfect linear relationship
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
    //     let (shift_est, scale_est) = least_squares(&x, &y).unwrap();
    //     assert_relative_eq!(shift_est, 0.0, epsilon = EPSILON);
    //     assert_relative_eq!(scale_est, 2.0, epsilon = EPSILON);

    //     // Test case 2: Linear relationship with intercept
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![3.0, 5.0, 7.0, 9.0, 11.0];
    //     let (shift_est, scale_est) = least_squares(&x, &y).unwrap();
    //     assert_relative_eq!(shift_est, 1.0, epsilon = EPSILON);
    //     assert_relative_eq!(scale_est, 2.0, epsilon = EPSILON);

    //     // Test case 3: Negative slope
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![10.0, 8.0, 6.0, 4.0, 2.0];
    //     let (shift_est, scale_est) = least_squares(&x, &y).unwrap();
    //     assert_relative_eq!(shift_est, 12.0, epsilon = EPSILON);
    //     assert_relative_eq!(scale_est, -2.0, epsilon = EPSILON);

    //     // Test case 4: Length mismatch
    //     let x = vec![1.0, 2.0, 3.0];
    //     let y = vec![2.0, 4.0];
    //     let result = least_squares(&x, &y);
    //     assert!(result.is_err());
    //     if let Err(LstsqError::LengthMismatch(x_len, y_len)) = result {
    //         assert_eq!(x_len, 3);
    //         assert_eq!(y_len, 2);
    //     } else {
    //         panic!("Expected LengthMismatch error");
    //     }

    //     // Test case 5: Constant x values (zero slope)
    //     let x = vec![5.0, 5.0, 5.0, 5.0];
    //     let y = vec![1.0, 2.0, 3.0, 4.0];
    //     let (shift_est, scale_est) = least_squares(&x, &y).unwrap();
    //     assert_relative_eq!(shift_est, 2.5, epsilon = EPSILON);
    //     assert_relative_eq!(scale_est, 0.0, epsilon = EPSILON);
    // }

    // #[test]
    // fn test_theil_sen() {
    //     // Test case 1: Perfect linear relationship
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
    //     let max_points = 5;
    //     let (shift_est, scale_est) = theil_sen(&x, &y, max_points).unwrap();
    //     assert_relative_eq!(shift_est, 0.0, epsilon = 0.01);
    //     assert_relative_eq!(scale_est, 0.5, epsilon = 0.01);

    //     // Test case 2: Linear relationship with outlier
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![3.0, 5.0, 20.0, 9.0, 11.0]; // Outlier at index 2
    //     let max_points = 5;
    //     let (shift_est, scale_est) = theil_sen(&x, &y, max_points).unwrap();
    //     // Theil-Sen should be robust to the outlier
    //     assert_relative_eq!(shift_est, 1.0, epsilon = 0.1);
    //     assert_relative_eq!(scale_est, 0.5, epsilon = 0.1);

    //     // Test case 3: Length mismatch
    //     let x = vec![1.0, 2.0, 3.0];
    //     let y = vec![2.0, 4.0];
    //     let max_points = 3;
    //     let result = theil_sen(&x, &y, max_points);
    //     assert!(result.is_err());
    //     if let Err(TheilSenError::LengthMismatch(x_len, y_len)) = result {
    //         assert_eq!(x_len, 3);
    //         assert_eq!(y_len, 2);
    //     } else {
    //         panic!("Expected LengthMismatch error");
    //     }

    //     // Test case 4: Subsampling
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    //     let y = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
    //     let max_points = 5; // Subsample to 5 points
    //     let (shift_est, scale_est) = theil_sen(&x, &y, max_points).unwrap();
    //     // Should still get reasonable estimates
    //     assert!(shift_est < 2.0 && shift_est > -2.0);
    //     assert!(scale_est > 0.4 && scale_est < 0.6);
    // }

    #[test]
    fn test_random_subset() {
        // Test case 1: Subset of the same size as the vector
        let vec_len = 5;
        let downsampled_len = 5;
        let subset = random_subset(vec_len, downsampled_len);
        assert_eq!(subset.len(), downsampled_len);
        // All elements should be unique and in range
        let mut seen = vec![false; vec_len];
        for &idx in &subset {
            assert!(idx < vec_len);
            assert!(!seen[idx]);
            seen[idx] = true;
        }
        assert!(seen.iter().all(|&x| x));

        // Test case 2: Smaller subset
        let vec_len = 10;
        let downsampled_len = 3;
        let subset = random_subset(vec_len, downsampled_len);
        assert_eq!(subset.len(), downsampled_len);
        // All elements should be unique and in range
        let mut seen = vec![false; vec_len];
        for &idx in &subset {
            assert!(idx < vec_len);
            assert!(!seen[idx]);
            seen[idx] = true;
        }
        assert_eq!(seen.iter().filter(|&&x| x).count(), downsampled_len);

        // Test case 3: Empty subset
        let vec_len = 5;
        let downsampled_len = 0;
        let subset = random_subset(vec_len, downsampled_len);
        assert_eq!(subset.len(), 0);
    }

    #[test]
    fn test_median() {
        // Test case 1: Odd number of elements
        let data = vec![1.0, 3.0, 5.0, 7.0, 9.0];
        let result = median(&data).unwrap();
        assert_eq!(result, 5.0);

        // Test case 2: Even number of elements
        let data = vec![1.0, 3.0, 5.0, 7.0];
        let result = median(&data).unwrap();
        assert_eq!(result, 4.0);

        // Test case 3: Empty vector
        let data: Vec<f32> = vec![];
        let result = median(&data);
        assert!(result.is_err());
        match result {
            Err(TheilSenError::MedianCalcEmptyVec) => {},
            _ => panic!("Expected MedianCalcEmptyVec error"),
        }

        // Test case 4: Single element
        let data = vec![42.0];
        let result = median(&data).unwrap();
        assert_eq!(result, 42.0);

        // Test case 5: Unsorted data (should work correctly only with sorted data)
        let data = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        // Our implementation expects sorted data, so the result will be incorrect
        let result = median(&data).unwrap();
        assert_eq!(result, 3.0); // This is element at index 2, not the actual median
    }
}