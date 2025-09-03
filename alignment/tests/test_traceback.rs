use alignment::core::refinement::refinement_core::bands::{Band, BandType};
use alignment::core::refinement::refinement_core::dp_algorithm::traceback::banded_traceback;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct JsonData {
    pub seq_band_start: Vec<usize>,
    pub seq_band_end: Vec<usize>,
    pub base_offsets: Vec<usize>,
    pub traceback: Vec<i32>,
    pub path: Vec<usize>,
}

fn load_json(path: &str) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    data
}


fn test_with_data_from(dirname: &str) {
    let dir = format!("tests/{}/refinement_dp/traceback", dirname);
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

        let mut path = vec![0; data.path.len()];

        banded_traceback(
            &mut path, 
            &band, 
            &data.base_offsets, 
            &data.traceback
        );

        assert_eq!(path, data.path);
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
