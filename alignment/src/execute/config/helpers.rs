/*!
 * This module contains helper functions used when transforming the CLI flags 
 * into a Config struct.
 */

pub fn calc_quantiles(min: f32, max: f32, steps: usize) -> Vec<f32> {
    if steps < 2 {
        return vec![min]; // or panic!("Steps must be >= 2");
    }

    let step_size = (max - min) / (steps - 1) as f32;
    (0..steps).map(|i| min + i as f32 * step_size).collect()
}


#[cfg(test)]
mod test {
    use approx::assert_relative_eq;

    use super::calc_quantiles;

    #[test]
    fn test_calc_quantiles() {
        let min = 0.05;
        let max = 0.95;
        let steps = 19;

        let quants = calc_quantiles(min, max, steps);
        let exp: Vec<f32> = vec![
            0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4,
            0.45, 0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8,
            0.85, 0.9, 0.95,
        ];
        for (l, r) in quants.iter().zip(exp.iter()) {
            assert_relative_eq!(l, r, epsilon=0.0001)
        }
    }
}