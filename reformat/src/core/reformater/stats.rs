use crate::error::core::reformat::StatError;

pub(super) fn mean_f32(values: &[f32]) -> Result<f32, StatError> {
    if values.is_empty() {
        return Err(StatError::VecEmpty);
    }
    let sum = values.iter().sum::<f32>();
    let n = values.len() as f32;
    Ok(sum / n)
}

pub(super) fn std_f32(values: &[f32]) -> Result<f32, StatError> {
    if values.is_empty() {
        return Err(StatError::VecEmpty);
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    let variance = values.iter()
        .map(|&el| {
            let diff = el - mean;
            diff * diff
        })
        .sum::<f32>() / values.len() as f32;
    Ok(variance.sqrt())
}

pub(super) fn median_f32(values: &[f32]) -> Result<f32, StatError> {
    if values.is_empty() {
        return Err(StatError::VecEmpty);
    }
    
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let n = sorted.len();
    if n % 2 == 1 {
        // Odd number of elements
        Ok(sorted[n / 2])
    } else {
        // Even number of elements - average the two middle values
        Ok((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    }
}

