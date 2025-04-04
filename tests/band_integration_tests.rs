use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize)]
struct JsonData {
    pub seq_to_sig_map: Vec<usize>,
    pub levels: Vec<f32>,
    pub band_half_width: usize,
    pub min_step: usize,
    pub sig_band_start: Vec<usize>,
    pub sig_band_end: Vec<usize>,
    pub seq_band_start: Vec<usize>,
    pub seq_band_end: Vec<usize>
}

fn load_test_data_signal(path: &str) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    data
}

use fishnet::refinement::refinement_core::bands::{Band, BandType};

#[test]
fn test_bands() {
    let dir = "tests/band";
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let file_name = path.unwrap().path();
        let path_str = file_name.to_str().unwrap();

        let data = load_test_data_signal(path_str);

        let mut band = Band::compute_signal_band(
            &data.seq_to_sig_map, 
            data.levels.len(), 
            data.band_half_width, 
            true
        ).unwrap();

        assert_eq!(*band.start(), data.sig_band_start, "Signal band start fail: {path_str}");
        assert_eq!(*band.end(), data.sig_band_end, "Signal band end fail: {path_str}");

        band.convert_to_sequence_band(2).unwrap();

        assert_eq!(*band.band_type(), BandType::SequenceBand);
        assert_eq!(*band.start(), data.seq_band_start, "Sequence band start fail: {path_str}");
        assert_eq!(*band.end(), data.seq_band_end, "Sequence band end fail: {path_str}");

    }
}