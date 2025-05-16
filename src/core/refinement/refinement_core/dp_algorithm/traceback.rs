use crate::{logger::get_log_vector_sample, core::refinement::refinement_core::bands::Band};

/// Performs traceback to reconstruct the optimal path (i.e. where each base starts)
///
/// This function implements the traceback step of the banded dynamic programming algorithm,
/// working backwards from the end of the signal to determine the precise boundaries
/// between bases in the sequence. It uses the traceback information computed during
/// the forward pass to reconstruct where each base starts in the signal.
///
/// # Arguments
///
/// * `path` - Mutable vector to be populated with start positions for each base. After execution:
///   - path\[0\] will contain the start position of the first base (always 0)
///   - path\[1..len-1\] will contain the start positions of subsequent bases
///   - path\[len-1\] will contain the end position of the final base (total signal length)
/// * `band` - The Band structure containing information about the allowed regions for each base
///   in the signal. This defines the search space constraints used during dynamic programming.
/// * `base_offsets` - Vector containing offsets into the traceback array for each base's information.
///   These offsets are necessary to locate the correct traceback information in the flattened array.
/// * `traceback` - Vector containing traceback information computed during the forward pass.
///   For each position, it stores the number of steps backward needed to reach the start of 
///   the current base. Organized as a flattened ragged array, with base_offsets indicating where
///   each base's information begins.
///
/// # Algorithm
///
/// The function works backwards through the sequence:
/// 1. Sets the start of the first base to position 0
/// 2. Sets the end of the last base to the end of the signal
/// 3. For each base (working backwards from the end):
///    - Uses the end position of the next base (already determined) as the lookup position
///    - Finds the corresponding traceback entry using band information and base offsets
///    - Uses the traceback value to determine how many steps backward to go to find the start
///      position of the current base
///    - Records this start position in the path vector
pub fn banded_traceback(
    path: &mut Vec<usize>,
    band: &Band,
    base_offsets: &Vec<usize>,
    traceback: &Vec<i32>
) {
    log::debug!(
        "banded_traceback input: path = {}, band start = {}, band end = {}, base_offsets = {}, traceback = {}",
        get_log_vector_sample(path, 10),
        get_log_vector_sample(band.start(), 10),
        get_log_vector_sample(band.end(), 10),
        get_log_vector_sample(base_offsets, 10),
        get_log_vector_sample(traceback, 10)
    );
    let seq_band_start = band.start();
    let seq_band_end = band.end();

    // Set start to 0 and end to final signal position
    path[0] = 0;
    let last_path_idx = path.len() - 1;
    path[last_path_idx] = seq_band_end[seq_band_end.len()-1];
    
    for base_idx in (1..last_path_idx).rev() {
        // Signal position to lookup for this traceback step
        let sig_lookup_pos = path[base_idx + 1] - 1;
    
        // Calculate offset into traceback array
        let base_offset = base_offsets[base_idx];
        let band_start = seq_band_start[base_idx];
        let traceback_idx = base_offset + (sig_lookup_pos - band_start);
        
        // Get number of steps backward to reach start of current base
        let next_sig_offset = traceback[traceback_idx];

        // Record position where base_idx starts
        path[base_idx] = sig_lookup_pos - (next_sig_offset as usize);
    } 

    log::debug!(
        "banded_traceback output: path = {}", 
        get_log_vector_sample(path, 10),
    );

}










#[cfg(test)]
mod test{
    use std::vec;

    use crate::core::refinement::refinement_core::bands::{Band, BandType};

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

        let band = Band::new(BandType::SequenceBand, seq_band_start, seq_band_end);
        banded_traceback(
            &mut path,
            &band,
            &base_offsets,
            &traceback
        );

        assert_eq!(path, vec![0,3,5,10]);
    }
}