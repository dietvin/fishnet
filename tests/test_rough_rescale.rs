use std::{fs::File, io::BufReader, path::PathBuf};

use approx::assert_relative_eq;
use fishnet::refinement::signal_map_refiner::rescale::{rough_rescale_lstsq, rough_rescale_theil_sen};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug)]
struct JsonData {
    shift: f32,
    scale: f32,
    seq_to_sig_map: Vec<usize>,
    int_seq: Vec<u32>,
    dacs: Vec<f32>,
    quants: Vec<f32>,
    clip_bases: usize,
    use_base_center: bool,
    levels: Vec<f32>,
    rough_rescale_method: String,
    new_shift: f32,
    new_scale: f32,
}

/// Loads test data from a JSON file for rough rescale implementations.
/// 
/// # Arguments
/// * `path` - Path to the JSON file containing the test data
/// 
/// # Returns
/// A tuple containing all parameters needed for testing rough rescale functions.
/// Panics if the file cannot be opened or parsed.
fn load_json(path: &str) -> JsonData {
    println!("{}", path);
    // Open the file and parse the JSON
    let file = File::open(path).expect("Failed to open test data file");
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    data
} 

fn test_with_data_from(dirname: &str) {
    let dir = format!("tests/{}/rough_rescale", dirname);

    let mut files= WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<PathBuf>>();

    files.sort();

    for file in files {
        let path_str = file.to_str().unwrap();

        let data = load_json(&path_str);

        let (shift, scale) = match data.rough_rescale_method.as_str() {
            "theil_sen" => {
                rough_rescale_theil_sen(
                    data.scale, 
                    data.shift, 
                    &data.seq_to_sig_map, 
                    &data.levels,
                    &data.dacs,
                    &data.quants, 
                    data.clip_bases, 
                    data.use_base_center
                ).unwrap()
            }
            "least_squares" => {
                rough_rescale_lstsq(
                    data.scale, 
                    data.shift, 
                    &data.seq_to_sig_map, 
                    &data.levels,
                    &data.dacs,
                    &data.quants, 
                    data.clip_bases, 
                    data.use_base_center
                ).unwrap()
            }
            _ => {
                panic!("Unexpected algorithm")
            }
        };

        assert_relative_eq!(shift, data.new_shift, epsilon=1.5);
        assert_relative_eq!(scale, data.new_scale, epsilon=1.5);
    }

}

#[test]
fn test_rough_rescale_querymap_theilsen() {
    test_with_data_from("test_data_querymap_theilsen");
}

#[test]
fn test_rough_rescale_refmap_theilsen() {
    test_with_data_from("test_data_refmap_theilsen");
}

#[test]
fn test_rough_rescale_querymap_leastsquares() {
    test_with_data_from("test_data_querymap_leastsquares");
}

#[test]
fn test_rough_rescale_refmap_leastsquares() {
    test_with_data_from("test_data_refmap_leastsquares");
}