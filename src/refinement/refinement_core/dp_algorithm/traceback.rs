/// Perform traceback to determine the path through a signal, reconstructing where each base starts.
/// 
/// This function performs the backtrace step of a banded dynamic programming algorithm,
/// working backwards from the end of the signal to determine where each base begins.
/// 
/// # Arguments
/// 
/// * `path` - Mutable vector to be populated with start positions. After execution:
///   - path[0..len-1] will contain the start position of each base (first base always starts at beginning)
///   - path[len-1] will contain the end position of the final base (signal length)
/// * `seq_band_start` - Vector containing the lower bound of the band for each base
/// * `seq_band_end` - Vector containing the upper bound of the band for each base
/// * `base_offsets` - Vector containing offsets into the traceback array for each base's information
/// * `traceback` - Vector containing the number of steps backward to reach the start of each base
///                 Organized as a flattened ragged array, with base_offsets indicating where
///                 each base's information begins
pub fn banded_traceback(
    path: &mut Vec<u32>,
    seq_band_start: &Vec<u32>,
    seq_band_end: &Vec<u32>,
    base_offsets: &Vec<u32>,
    traceback: &Vec<i32>
) {
    // Set start to 0 and end to final signal position
    path[0] = 0;
    let last_path_idx = path.len() - 1;
    path[last_path_idx] = seq_band_end[seq_band_end.len()-1] as u32;
    
    for base_idx in (1..last_path_idx).rev() {
        // Signal position to lookup for this traceback step
        let sig_lookup_pos = path[base_idx + 1] - 1;
    
        // Calculate offset into traceback array
        let base_offset = base_offsets[base_idx] as usize;
        let band_start = seq_band_start[base_idx];
        let traceback_idx = base_offset + (sig_lookup_pos - band_start) as usize;
        
        // Get number of steps backward to reach start of current base
        let next_sig_offset = traceback[traceback_idx];

        // Record position where base_idx starts
        path[base_idx] = (sig_lookup_pos as u32) - (next_sig_offset as u32);
    } 
}










#[cfg(test)]
mod test{
    use std::vec;

    use super::banded_traceback;

    #[test]
    fn test_simple() {
        let n_bases = 3;
        let mut path = vec![0;n_bases+1];
        
        let seq_band_start = vec![0,3,5];
        let seq_band_end = vec![3,5,10];

        let base_offsets = vec![0,3,5];
        
        let traceback = vec![
            0, 1, 2,
                     0, 1,
                           0, 1, 2, 3, 4
        ];

        banded_traceback(
            &mut path,
            &seq_band_start,
            &seq_band_end,
            &base_offsets,
            &traceback
        );

        assert_eq!(path, vec![0,3,5,10]);
    }
}