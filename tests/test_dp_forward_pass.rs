use approx::assert_relative_eq;
use fishnet::refinement::refinement_core::bands::{Band, BandType};
use fishnet::refinement::refinement_core::dp_algorithm::forward_pass::forward_pass;
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
    pub all_scores_len: usize,
    pub traceback_len: usize,
    pub signal: Vec<f32>,
    pub levels: Vec<f32>,
    pub seq_band_start: Vec<usize>,
    pub seq_band_end: Vec<usize>,
    pub base_offsets: Vec<usize>,
    pub short_dwell_penalty: SdParams,
    pub core_method: String,
    pub all_scores_result: Vec<f32>,
    pub traceback_result: Vec<i32>
}

fn load_json(path: &str) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    data
}


fn test_with_data_from(dirname: &str) {
    let dir = format!("tests/{}/refinement_dp/forward_pass", dirname);
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let file_name = path.unwrap().path();
        let path_str = file_name.to_str().unwrap();

        let data = load_json(path_str);
        let band = Band::new(
            BandType::SequenceBand, 
            data.seq_band_start, 
            data.seq_band_end
        );
        
        let mut scores = vec![f32::INFINITY; data.all_scores_len];
        let mut traceback = vec![0; data.traceback_len];

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

        forward_pass(
            &mut scores, 
            &mut traceback, 
            &data.signal, 
            &data.levels, 
            &band, 
            &data.base_offsets, 
            &method
        );

        for (r, e) in scores.iter().zip(data.all_scores_result.iter()) {
            assert_relative_eq!(r, e, epsilon=0.01);
        }
        assert_eq!(traceback, data.traceback_result);
    }
}

#[test]
fn test_forward_pass_querymap_theilsen() {
    test_with_data_from("test_data_querymap_theilsen");
}

#[test]
fn test_forward_pass_refmap_theilsen() {
    test_with_data_from("test_data_refmap_theilsen");
}

#[test]
fn test_forward_pass_querymap_leastsquares() {
    test_with_data_from("test_data_querymap_leastsquares");
}

#[test]
fn test_forward_pass_refmap_leastsquares() {
    test_with_data_from("test_data_refmap_leastsquares");
}
