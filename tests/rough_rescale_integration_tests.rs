use fishnet::refinement::signal_map_refiner::rescale::{rough_rescale_lstsq, rough_rescale_theil_sen};

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

use approx::assert_relative_eq;


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
    result: Vec<f32>,
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
    (f32, f32)      // expected (shift, scale)
) {
    // Open the file and parse the JSON
    let file = File::open(path).expect("Failed to open test data file");
    let reader = BufReader::new(file);
    let data: RoughRescaleData = serde_json::from_reader(reader).expect("Failed to parse JSON");

    let result = (
        data.result[0],
        data.result[1],
    ); 
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
        result
    )
} 

fn get_expected_actual(dir: &str, alg: &str) -> Vec<((f32, f32), (f32, f32))> {
    let mut results = Vec::new();
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let path = path.unwrap().path();
        let path_str = path.to_str().unwrap();
        println!("{}", path_str);
        let (scale,
            shift,
            seq_to_signal_map,
            levels,
            signal,
            quantiles,
            clip_bases,
            use_base_center,
            (exp_shift, exp_scale)) = load_test_data(path_str);
        
        let (shift, scale) = match alg {
            "theil_sen" => {
                rough_rescale_theil_sen(scale, shift, &seq_to_signal_map, &levels, &signal, &quantiles, clip_bases, use_base_center).unwrap()
            }
            "least_squares" => {
                rough_rescale_lstsq(scale, shift, &seq_to_signal_map, &levels, &signal, &quantiles, clip_bases, use_base_center).unwrap()
            }
            _ => {
                panic!("Unexpected algorithm")
            }
        };
        results.push(((shift, scale), (exp_shift, exp_scale)));
    }
    results
}

/// Test rough least squares rough rescaling with clip_bases=0 & use_base_center=true
#[test]
fn test_ls_00_t() {
    let input_dir = "tests/rough_rescale/ls_0_t";
    let results = get_expected_actual(input_dir, "least_squares");

    for (calculated, expected) in results {
            assert_relative_eq!(calculated.0, expected.0, epsilon=0.9);
            assert_relative_eq!(calculated.1, expected.1, epsilon=0.9);
    }
}

/// Test rough least squares rough rescaling with clip_bases=0 & use_base_center=false
#[test]
fn test_ls_00_f() {
    // let input_dir = "tests/rough_rescale/ls_0_f";
    let input_dir = "tests/rough_rescale/single_read";

    let results = get_expected_actual(input_dir, "least_squares");

    for (calculated, expected) in results {
            assert_relative_eq!(calculated.0, expected.0, epsilon=0.9);
            assert_relative_eq!(calculated.1, expected.1, epsilon=0.9);
    }
}

/// Test rough least squares rough rescaling with clip_bases=10 & use_base_center=true
#[test]
fn test_ls_10_t() {
    let input_dir = "tests/rough_rescale/ls_10_t";
    let results = get_expected_actual(input_dir, "least_squares");

    for (calculated, expected) in results {
            assert_relative_eq!(calculated.0, expected.0, epsilon=0.9);
            assert_relative_eq!(calculated.1, expected.1, epsilon=0.9);
    }
}

/// Test rough theil sen rough rescaling with clip_bases=0 & use_base_center=true
#[test]
fn test_ts_00_t() {
    let input_dir = "tests/rough_rescale/ts_0_t";
    let results = get_expected_actual(input_dir, "theil_sen");

    for (calculated, expected) in results {
            assert_relative_eq!(calculated.0, expected.0, epsilon=0.9);
            assert_relative_eq!(calculated.1, expected.1, epsilon=0.9);
    }
}
/// Test rough theil sen rough rescaling with clip_bases=0 & use_base_center=false
#[test]
fn test_ts_00_f() {
    let input_dir = "tests/rough_rescale/ts_0_f";
    let results = get_expected_actual(input_dir, "theil_sen");

    for (calculated, expected) in results {
            assert_relative_eq!(calculated.0, expected.0, epsilon=0.9);
            assert_relative_eq!(calculated.1, expected.1, epsilon=0.9);
    }
}
/// Test rough theil sen rough rescaling with clip_bases=10 & use_base_center=true
#[test]
fn test_ts_10_t() {
    let input_dir = "tests/rough_rescale/ts_10_t";
    let results = get_expected_actual(input_dir, "theil_sen");

    for (calculated, expected) in results {
            assert_relative_eq!(calculated.0, expected.0, epsilon=0.9);
            assert_relative_eq!(calculated.1, expected.1, epsilon=0.9);
    }
}