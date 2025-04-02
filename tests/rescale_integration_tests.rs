use approx::assert_relative_eq;
use fishnet::refinement::signal_map_refiner::rescale::rescale;
use fishnet::refinement::signal_map_refiner::settings::RescaleAlgo;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;


#[derive(Debug, Serialize, Deserialize)]
struct JsonRescale {
    pub levels: Vec<f32>,
    pub dacs: Vec<f32>,
    pub shift: f32,
    pub scale: f32,
    pub seq_to_sig_map: Vec<usize>,
    pub dwell_filter_pctls: (f32, f32),
    pub min_abs_level: f32,
    pub edge_filter_bases: usize,
    pub min_levels: usize,
    pub new_shift: f32,
    pub new_scale: f32
}

fn load_test_data_rescale(path: &str) -> JsonRescale {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonRescale = serde_json::from_reader(reader).unwrap();
    data
}

#[test]
fn test_rescale_default_settings() {
    let rescale_algo = RescaleAlgo::TheilSen { 
        dwell_filter_lower_percentile: 0.1, 
        dwell_filter_upper_percentile: 0.9, 
        min_abs_level: 0.2, 
        n_bases_truncate: 10, 
        min_num_filtered_levels: 10, 
        max_points: 100000000 
    };

    let dir = "tests/rescale";
    let paths = std::fs::read_dir(dir).unwrap();

    for path in paths {
        let file_name = path.unwrap().path();
        let path_str = file_name.to_str().unwrap();

        let data = load_test_data_rescale(path_str);

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