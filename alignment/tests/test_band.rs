use alignment::core::refinement::band::sequence_band::SequenceBand;
use alignment::core::refinement::band::signal_band::SignalBand;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize)]
struct JsonData {
    pub seq_to_sig_map: Vec<usize>,
    pub levels: Vec<f32>,
    pub band_half_width: usize,
    pub adjust_band_min_step: usize,
    pub sig_band_start: Vec<usize>,
    pub sig_band_end: Vec<usize>,
    pub seq_band_start: Vec<usize>,
    pub seq_band_end: Vec<usize>
}

fn load_json(path: &str) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    data
}

fn test_with_data_from(dirname: &str) {
    let dir = format!("tests/{}/refinement_dp/band", dirname);
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let file_name = path.unwrap().path();
        let path_str = file_name.to_str().unwrap();

        let data = load_json(path_str);

        let signal_band = SignalBand::new(
            &data.seq_to_sig_map,
            data.levels.len(),
            data.band_half_width,
            true
        ).unwrap();
        
        assert_eq!(signal_band.start, data.sig_band_start, "Signal band start fail: {path_str}");
        assert_eq!(signal_band.end, data.sig_band_end, "Signal band end fail: {path_str}");

        let sequence_band = SequenceBand::from_signal_band(
            signal_band,
            data.adjust_band_min_step
        ).unwrap();

        assert_eq!(*sequence_band.start(), data.seq_band_start, "Sequence band start fail: {path_str}");
        assert_eq!(*sequence_band.end(), data.seq_band_end, "Sequence band end fail: {path_str}");
    }

}

#[test]
fn test_bands_querymap_theilsen() {
    test_with_data_from("test_data_querymap_theilsen");
}

#[test]
fn test_bands_refmap_theilsen() {
    test_with_data_from("test_data_refmap_theilsen");
}

#[test]
fn test_bands_querymap_leastsquares() {
    test_with_data_from("test_data_querymap_leastsquares");
}

#[test]
fn test_bands_refmap_leastsquares() {
    test_with_data_from("test_data_refmap_leastsquares");
}

// #[test]
// fn test_bands_refmap_leastsquares() {
//     test_with_data_from("/home/vincent/projects/resquiggle_tool/remora_debug/00_data_extraction/test/refinement_dp/band");
// }