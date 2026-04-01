use alignment::core::refinement::band::sequence_band::SequenceBand;
use alignment::core::refinement::dp::banded_db;
use alignment::core::refinement::dp::forward_step::dwell_penalty::DwellPenalty;
use alignment::core::refinement::dp::forward_step::viterbi::Viterbi;
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

        let band = SequenceBand::from_existing_vecs(
            data.sequence_band_start,
            data.sequence_band_end
        );

        let refined_map = match data.core_method.as_str() {
            // Hardcoded default values used during testing, because 
            // I exported the already calulated array
            "dwell_penalty" => {
                let algo = DwellPenalty::new(
                4.0, // data.short_dwell_penalty.target, 
                3.0, // data.short_dwell_penalty.limit, 
                0.5 // data.short_dwell_penalty.weight 
                );

                banded_db(
                    &data.signal,
                    &data.levels,
                    &band,
                    &algo
                )
            }
            "Viterbi" => {
                let algo = Viterbi;

                banded_db(
                    &data.signal,
                    &data.levels,
                    &band,
                    &algo
                )
            },
            _ => panic!("Unknown core_method")
        };

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