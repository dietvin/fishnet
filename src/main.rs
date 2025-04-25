mod error;
mod loader;
mod alignment;
mod refinement;
mod logger;

use alignment::aligned_read::AlignedRead;
use fishnet::logger::setup_logger;


use loader::bam::BamFileLazy;
use loader::pod5::Pod5Index;
use refinement::{settings::{RefineAlgo, RefineSettings, RescaleAlgo, RoughRescaleAlgo, WhichToRefine}, signal_map_refiner::SigMapRefiner};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use walkdir::WalkDir;
use std::path::{Path, PathBuf};


#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct SdParams {
    target: f32,
    limit: f32,
    weight: f32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Parameters {
    do_rough_rescale: bool,
    scale_iters: i32,
    algo: String,
    half_bandwidth: usize,
    sd_params: SdParams,
    do_fix_gauge: bool,
    rough_rescale_method: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct JsonData {
    read_id: String,
    parameters: Parameters,
    ref_mapping: bool,
    unrefined_alignment: Vec<usize>,
    refined_alignment: Vec<usize>
}

fn load_json(path: &str) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    data
}

fn refine_settings_from_data(data: &JsonData) -> RefineSettings {

    let which_map_to_refine = match data.ref_mapping {
        true => WhichToRefine::Reference,
        false => WhichToRefine::Query
    };

    let refinement_algo = match data.parameters.algo.as_str() {
        "Viterbi" => RefineAlgo::Viterbi,
        "dwell_penalty" => RefineAlgo::DwellPenalty { 
            target: data.parameters.sd_params.target, 
            limit: data.parameters.sd_params.limit, 
            weight: data.parameters.sd_params.weight
        },
        _ => panic!("Unknown algo value")
    };

    let n_refinement_iters = if data.parameters.scale_iters == -1 {
        0
    } else {
        data.parameters.scale_iters as usize
    };

    let rough_rescale_algo = match data.parameters.do_rough_rescale {
        false => RoughRescaleAlgo::NoRoughRescaling,
        true => {
            match data.parameters.rough_rescale_method.as_str() {
                "least_squares" => RoughRescaleAlgo::LeastSquares { 
                    quantiles: vec![0.05, 0.1 , 0.15, 0.2 , 0.25, 0.3 , 0.35, 0.4 , 0.45, 0.5 , 0.55, 0.6 , 0.65, 0.7 , 0.75, 0.8 , 0.85, 0.9 , 0.95], 
                    clip_bases: 10, 
                    use_base_center: true },
                "theil_sen" => RoughRescaleAlgo::TheilSen { 
                    quantiles: vec![0.05, 0.1 , 0.15, 0.2 , 0.25, 0.3 , 0.35, 0.4 , 0.45, 0.5 , 0.55, 0.6 , 0.65, 0.7 , 0.75, 0.8 , 0.85, 0.9 , 0.95], 
                    clip_bases: 10, 
                    use_base_center: true },
                _ => panic!("Unknown rough_rescale_algo value")
            }
        }
        
    };

    let settings = RefineSettings::custom(
        which_map_to_refine, 
        refinement_algo, 
        n_refinement_iters, 
        data.parameters.half_bandwidth, 
        2, 
        RescaleAlgo::TheilSen { 
            dwell_filter_lower_percentile: 0.1,
            dwell_filter_upper_percentile: 0.9,
            min_abs_level: 0.2,
            n_bases_truncate: 10,
            min_num_filtered_levels: 10,        
            max_points: 100000000 
        }, 
        rough_rescale_algo, 
        data.parameters.do_fix_gauge
    );

    settings
}


pub fn test_vectors_x_percent_equal(vec1: &[usize], vec2: &[usize], x: f32) -> (bool, f32, usize) {
    if vec1.len() != vec2.len() {
        return (false, 0.0, vec1.len() + vec2.len());
    }
    
    // Count exact matches
    let matches = vec1.iter().zip(vec2.iter()).filter(|&(a, b)| a == b).count();
    let total = vec1.len();
    let percentage = (matches as f32 / total as f32) * 100.0;
    
    // Check if at least 99% match
    let passed = percentage >= x;
    let differences = total - matches;
    
    (passed, percentage, differences)
}


fn main() {
    // setup_logger(
    //     "log.txt", 
    //     log::LevelFilter::Info, 
    //     vec![("fishnet::refinement::signal_map_refiner::rescale", log::LevelFilter::Trace)],
    //     false
    // ).unwrap();

    let path: &str = "example_data/can_reads.pod5";
    let paths = vec![path.to_string()];
    let index = Pod5Index::from_files(&paths).unwrap();
    let mut pod5_file = index.files().next().unwrap().unwrap();

    let path = "example_data/can_mappings.bam";
    let mut bam_file = BamFileLazy::new(path).unwrap();

    let dir = "tests/full_implementation";

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
        let read_id = data.read_id.clone();

        let bam_read = bam_file.get(&read_id).unwrap();
        let pod5_read = pod5_file.get_mut(&read_id).unwrap();

        let mut aligned_read = AlignedRead::new(
            pod5_read, 
            &bam_read, 
            false
        ).unwrap();

        let alignment = match data.ref_mapping {
            false => {
                aligned_read.align_query_to_signal().unwrap();
                aligned_read.query_to_signal().unwrap()
            }
            true => {
                aligned_read.align_query_to_signal().unwrap();
                aligned_read.align_reference_to_signal().unwrap();
                aligned_read.reference_to_signal().unwrap()
            }
        };

        let settings = refine_settings_from_data(&data);
        let mut sig_map_refiner = SigMapRefiner::new(
            "example_data/levels.txt", 
            &aligned_read, 
            settings.clone()
        ).unwrap();

        sig_map_refiner.start().unwrap();


        let refined_alignment = match data.ref_mapping {
            false => sig_map_refiner.refined_query_to_sig().unwrap(),
            true => sig_map_refiner.refined_ref_to_sig().unwrap(),
        };

        let (passes_test, pct_mismatch, n_differences) = test_vectors_x_percent_equal(refined_alignment, &data.refined_alignment, 90.0);
        println!("{} | {} | Pct match: {} ({} different positions)", path_str, passes_test, pct_mismatch, n_differences);
    }
}