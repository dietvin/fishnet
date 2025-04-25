use fishnet::refinement::refinement_core::bands::{Band, BandType};
use fishnet::refinement::refinement_core::dp_algorithm::banded_dp;
use fishnet::refinement::settings::RefineAlgo;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct SdParams {
    target: f32,
    limit: f32,
    weight: f32
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonData {
     pub signal: Vec<f32>,
     pub levels: Vec<f32>,
     pub sequence_band_start: Vec<usize>,
     pub sequence_band_end: Vec<usize>,
     pub short_dwell_penalty: SdParams,
     pub core_method: String, 
     pub all_scores: Vec<f32>,
     pub path: Vec<usize>,
     pub traceback: Vec<i32>
}

fn load_json(path: &str) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    data
}

fn test_with_data_from(dirname: &str) {
    let dir = format!("tests/{}/refinement_dp/banded_dp", dirname);
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let file_name = path.unwrap().path();
        let path_str = file_name.to_str().unwrap();

        let data = load_json(path_str);
        let band = Band::new(
            BandType::SequenceBand, 
            data.sequence_band_start, 
            data.sequence_band_end
        );
        
        let method = match data.core_method.as_str() {
            // Hardcoded default values used during testing, because 
            // I exported the already calulated array
            "dwell_penalty" => RefineAlgo::DwellPenalty { 
                target: 4.0, // data.short_dwell_penalty.target, 
                limit: 3.0, // data.short_dwell_penalty.limit, 
                weight: 0.5 // data.short_dwell_penalty.weight 
            },
            "Viterbi" => RefineAlgo::Viterbi,
            _ => panic!("Unknown core_method")
        };

        let refined_map = banded_dp(
            &data.signal, 
            &data.levels, 
            &band,
            &method
        );

        assert_eq!(refined_map, data.path, "Path differs from expected for: {path_str}");
    }
}

#[test]
fn test_banded_dp_querymap_theilsen() {
    test_with_data_from("test_data_querymap_theilsen");
}

#[test]
fn test_banded_dp_refmap_theilsen() {
    test_with_data_from("test_data_refmap_theilsen");
}

#[test]
fn test_banded_dp_querymap_leastsquares() {
    test_with_data_from("test_data_querymap_leastsquares");
}

#[test]
fn test_banded_dp_refmap_leastsquares() {
    test_with_data_from("test_data_refmap_leastsquares");
}