use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Serialize, Deserialize, Debug)]
struct RoughRescaleParams {
    shift: f32,
    scale: f32,
    seq_to_sig_map: Vec<usize>,
    dacs: Vec<f32>,
    quants: Vec<f32>,
    clip_bases: usize,
    use_base_center: bool,
    levels: Vec<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RoughRescaleData {
    params: RoughRescaleParams,
}

/// Loads test data from a JSON file for rough rescale implementations.
/// 
/// # Arguments
/// * `path` - Path to the JSON file containing the test data
/// 
/// # Returns
/// A tuple containing all parameters needed for testing rough rescale functions.
/// Panics if the file cannot be opened or parsed.
fn load_test_data(path: &str) -> (
    f32,            // scale
    f32,            // shift
    Vec<usize>,     // seq_to_signal_map
    Vec<f32>,       // levels
    Vec<f32>,       // signal (dacs)
    Vec<f32>,       // quantiles
    usize,          // clip_bases
    bool,           // use_base_center
) {
    // Open the file and parse the JSON
    let file = File::open(path).expect("Failed to open test data file");
    let reader = BufReader::new(file);
    let data: RoughRescaleData = serde_json::from_reader(reader).expect("Failed to parse JSON");
    
    // Return the tuple with all parameters
    (
        data.params.scale,
        data.params.shift,
        data.params.seq_to_sig_map,
        data.params.levels,
        data.params.dacs,
        data.params.quants,
        data.params.clip_bases,
        data.params.use_base_center,
    )
} 

