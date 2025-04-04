use approx::assert_relative_eq;
use fishnet::refinement::refinement_core::bands::{Band, BandType};
use fishnet::refinement::refinement_core::dp_algorithm::forward_pass::forward_pass;
use fishnet::refinement::refinement_core::dp_algorithm::forward_step::forward_step_viterbi;
use fishnet::refinement::refinement_core::dp_algorithm::banded_dp;
use fishnet::refinement::signal_map_refiner::settings::RefineAlgo;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize)]
struct JsonDataBandedDp {
     pub signal: Vec<f32>,
     pub levels: Vec<f32>,
     pub sequence_band_start: Vec<usize>,
     pub sequence_band_end: Vec<usize>,
     pub short_dwell_penalty: Vec<f32>,
     pub core_method: String, 
     pub all_scores: Vec<f32>,
     pub path: Vec<usize>,
     pub traceback: Vec<i32>
}


fn load_banded_dp(path: &str) -> JsonDataBandedDp {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonDataBandedDp = serde_json::from_reader(reader).unwrap();

    data
}


#[test]
fn test_dp_algorithm() {
    let dir = "tests/banded_dp";
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let file_name = path.unwrap().path();
        let path_str = file_name.to_str().unwrap();

        let data = load_banded_dp(path_str);
        let band = Band::new(
            BandType::SequenceBand, 
            data.sequence_band_start, 
            data.sequence_band_end
        );
        
        let refined_map = banded_dp(
            &data.signal, 
            &data.levels, 
            &band, 
            &RefineAlgo::DwellPenalty { target: 4.0, limit: 3.0, weight: 0.5 }
        );

        assert_eq!(refined_map, data.path);
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct JsonDataForwardPass {
    pub all_scores_len: usize,
    pub traceback_len: usize,
    pub signal: Vec<f32>,
    pub levels: Vec<f32>,
    pub seq_band_start: Vec<usize>,
    pub seq_band_end: Vec<usize>,
    pub base_offsets: Vec<usize>,
    pub short_dwell_penalty: Vec<f32>,
    pub core_method: String,
    pub all_scores_result: Vec<f32>,
    pub traceback_result: Vec<i32>
}


fn load_forward_pass(path: &str) -> JsonDataForwardPass {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonDataForwardPass = serde_json::from_reader(reader).unwrap();

    data
}


#[test]
fn test_forward_pass() {
    let dir = "tests/forward_pass";
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let file_name = path.unwrap().path();
        let path_str = file_name.to_str().unwrap();

        let data = load_forward_pass(path_str);
        let band = Band::new(
            BandType::SequenceBand, 
            data.seq_band_start, 
            data.seq_band_end
        );
        
        let mut scores = vec![0.0; data.all_scores_len];
        let mut traceback = vec![0; data.traceback_len];

        let method = match data.core_method.as_str() {
            "dwell_penalty" => RefineAlgo::DwellPenalty { target: 4.0, limit: 3.0, weight: 0.5 },
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


#[derive(Debug, Serialize, Deserialize)]
struct JsonDataForwardStepViterbi {
    pub curr_scores_len: usize,
    pub curr_tb_len: usize,
    pub prev_scores: Vec<f32>,
    pub curr_level: f32,
    pub curr_signal: Vec<f32>,
    pub band_start_diff: usize,
    pub curr_scores_res: Vec<f32>,
    pub curr_tb_res: Vec<i32>
}


fn load_forward_step_viterbi(path: &str) -> JsonDataForwardStepViterbi {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonDataForwardStepViterbi = serde_json::from_reader(reader).unwrap();

    data
}

#[test]
fn test_forward_step_viterbi() {
    let dir = "tests/forward_step_viterbi";
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let file_name = path.unwrap().path();
        let path_str = file_name.to_str().unwrap();
        println!("{path_str}");

        let data = load_forward_step_viterbi(path_str);
        
        let mut scores = vec![0.0; data.curr_scores_len];
        let mut traceback = vec![0; data.curr_tb_len];

        forward_step_viterbi(
            &mut scores, 
            &mut traceback, 
            &data.prev_scores, 
            data.curr_level, 
            &data.curr_signal, 
            data.band_start_diff
        );

        for (r, e) in scores.iter().zip(data.curr_scores_res.iter()) {
            assert_relative_eq!(r, e, epsilon=0.01);
        }
        assert_eq!(traceback, data.curr_tb_res);
    }
}
