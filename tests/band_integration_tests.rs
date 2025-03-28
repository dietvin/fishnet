use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize)]
struct Arguments {
    bps: Vec<usize>,
    levels: Vec<f32>,
    bhw: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonData {
    pub timestamp: String,
    pub function: String,
    pub arguments: Arguments,
    pub result: Vec<Vec<usize>>
}

fn load_test_data(path: &str) -> (
    Vec<usize>, // sequence_to_signal_map
    Vec<f32>, // expected_levels
    usize, // half_bandwidth
    Vec<usize>, // start
    Vec<usize> // end
) {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    let sequence_to_signal_map = data.arguments.bps;
    let expected_levels = data.arguments.levels;
    let half_bandwidth = data.arguments.bhw;
    let start = data.result[0].clone();
    let end = data.result[1].clone();

    (
        sequence_to_signal_map,
        expected_levels,
        half_bandwidth,
        start,
        end
    )
}

use fishnet::refinement::refinement_core::bands::Band;

#[test]
fn test_compute_sig_band() {
    let dir = "tests/signal_bands/compute_sig_band";
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let file_name = path.unwrap().path();
        let path_str = file_name.to_str().unwrap();

        let (
            sequence_to_signal_map, 
            expected_levels, 
            half_bandwidth, 
            start, 
            end
        ) = load_test_data(path_str);

        let band = Band::compute_signal_band(
            &sequence_to_signal_map, 
            expected_levels.len(), 
            half_bandwidth, 
            true
        ).unwrap();

        assert_eq!(*band.start(), start);
        assert_eq!(*band.end(), end);
    }

}