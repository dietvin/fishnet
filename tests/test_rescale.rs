use approx::assert_relative_eq;
use fishnet::refinement::settings::RescaleAlgo;
use fishnet::refinement::signal_map_refiner::rescale::rescale;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;


#[derive(Debug, Serialize, Deserialize)]
struct JsonData {
    pub levels: Vec<f32>,
    pub dacs: Vec<f32>,
    pub shift: f32,
    pub scale: f32,
    pub seq_to_sig_map: Vec<usize>,
    pub new_shift: f32,
    pub new_scale: f32
}

fn load_json(path: &str) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();
    data
}

fn test_with_data_from(dirname: &str) {
    let dir = format!("tests/{}/rescale", dirname);

    let rescale_algo = RescaleAlgo::TheilSen { 
        dwell_filter_lower_percentile: 0.1, 
        dwell_filter_upper_percentile: 0.9, 
        min_abs_level: 0.2, 
        n_bases_truncate: 10, 
        min_num_filtered_levels: 10, 
        max_points: 100000000 
    };

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

        let (new_shift, new_scale) = rescale(
            data.scale, 
            data.shift, 
            &data.seq_to_sig_map, 
            &data.levels, 
            &data.dacs, 
            &rescale_algo
        ).unwrap();
        
        assert_relative_eq!(new_shift, data.new_shift, epsilon=0.1);
        assert_relative_eq!(new_scale, data.new_scale, epsilon=0.1);
    }
}

#[test]
fn test_rescale_querymap_theilsen() {
    test_with_data_from("test_data_querymap_theilsen");
}

#[test]
fn test_rescale_refmap_theilsen() {
    test_with_data_from("test_data_refmap_theilsen");
}

#[test]
fn test_rescale_querymap_leastsquares() {
    test_with_data_from("test_data_querymap_leastsquares");
}

#[test]
fn test_rescale_refmap_leastsquares() {
    test_with_data_from("test_data_refmap_leastsquares");
}