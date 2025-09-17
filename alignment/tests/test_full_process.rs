use alignment::core::refinement::kmer_table::KmerTable;
use alignment::execute::config::refinement_config::{RefineAlgo, RescaleAlgo, RoughRescaleAlgo, WhichToRefine, RefineSettings};
use alignment::core::alignment::aligned_read::AlignedRead;
use alignment::core::loader::bam::BamFileLazy;
use alignment::core::refinement::signal_map_refiner::SigMapRefiner;
use pod5_reader_api::file::Pod5File;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fs::File;
use std::io::BufReader;
use walkdir::WalkDir;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct SdParams {
    target: f32,
    limit: f32,
    weight: f32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct JsonData {
    read_id: String,
    reference_mapping: bool,
    kmer_model_filename: String,
    do_rough_rescale: bool,
    scale_iters: i32,
    algo: String,
    half_bandwidth: usize,
    sd_params: SdParams,
    do_fix_guage: bool,
    rough_rescale_method: String,
    unrefined_map: Vec<usize>,
    refined_map: Vec<usize>
}

fn load_json(path: &str) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    data
}

fn refine_settings_from_data(data: &JsonData) -> RefineSettings {

    let which_map_to_refine = match data.reference_mapping {
        true => WhichToRefine::Reference,
        false => WhichToRefine::Query
    };

    let refinement_algo = match data.algo.as_str() {
        "Viterbi" => RefineAlgo::Viterbi,
        "dwell_penalty" => RefineAlgo::DwellPenalty { 
            target: data.sd_params.target, 
            limit: data.sd_params.limit, 
            weight: data.sd_params.weight
        },
        _ => panic!("Unknown algo value")
    };

    let n_refinement_iters = if data.scale_iters == -1 {
        0
    } else {
        data.scale_iters as usize
    };

    let rough_rescale_algo = match data.do_rough_rescale {
        false => RoughRescaleAlgo::NoRoughRescaling,
        true => {
            match data.rough_rescale_method.as_str() {
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
        data.half_bandwidth, 
        2, 
        RescaleAlgo::TheilSen { 
            dwell_filter_lower_percentile: 0.1,
            dwell_filter_upper_percentile: 0.9,
            min_abs_level: 0.2,
            n_bases_truncate: 10,
            min_num_filtered_levels: 10,        
            max_points: 1000000000 
        }, 
        rough_rescale_algo, 
        data.do_fix_guage
    );

    settings
}

/// Tests if two vectors have at least 95% of their values exactly matching.
///
/// # Arguments
///
/// * `vec1` - First vector of usize values
/// * `vec2` - Second vector of usize values
///
/// # Returns
///
/// A tuple containing:
/// * Whether the test passed (true if ≥95% match)
/// * The percentage of matching elements
/// * The count of differences
pub fn test_vectors_95_percent_equal(vec1: &[usize], vec2: &[usize]) -> (bool, f32, usize) {
    if vec1.len() != vec2.len() {
        return (false, 0.0, vec1.len() + vec2.len());
    }
    
    // Count exact matches
    let matches = vec1.iter().zip(vec2.iter()).filter(|&(a, b)| a == b).count();
    let total = vec1.len();
    let percentage = (matches as f32 / total as f32) * 100.0;
    
    // Check if at least 99% match
    let passed = percentage >= 95.0;
    let differences = total - matches;
    
    (passed, percentage, differences)
}

fn test_with_data_from(dirname: &str) {
    let dir = format!("tests/{}/full_process", dirname);

    let path = PathBuf::from("../example_data/can_reads.pod5");
    let mut pod5_file = Pod5File::new(&path).unwrap();

    let path = "../example_data/can_mappings.bam";
    let mut bam_file = BamFileLazy::new(&PathBuf::from(path)).unwrap();

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

        let mut bam_read = bam_file.get(&read_id).unwrap();
        let read_uuid = Uuid::try_from(read_id).unwrap();
        let mut pod5_read = pod5_file.get(&read_uuid).unwrap();

        let mut aligned_read = AlignedRead::new(
            &mut pod5_read, 
            &mut bam_read, 
            false
        ).unwrap();

        let alignment = match data.reference_mapping {
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

        assert_eq!(*alignment, data.unrefined_map, "Unrefined: {}", path_str);

        let settings = refine_settings_from_data(&data);
        let kmer_table = KmerTable::new(&PathBuf::from("../example_data/levels.txt")).unwrap();
        let mut sig_map_refiner = SigMapRefiner::new(
            &kmer_table, 
            &mut aligned_read, 
            &settings
        ).unwrap();

        sig_map_refiner.start().unwrap();

        let refined_alignment = match data.reference_mapping {
            false => sig_map_refiner.refined_query_to_sig().unwrap(),
            true => sig_map_refiner.refined_ref_to_sig().unwrap(),
        };

        let (passes_test, pct_mismatch, n_differences) = test_vectors_95_percent_equal(refined_alignment, &data.refined_map);
        assert!(passes_test, "{} | Pct mismatch: {} ({} different positions)", path_str, pct_mismatch, n_differences);
    }
}

#[test]
fn test_full_process_querymap_theilsen() {
    test_with_data_from("test_data_querymap_theilsen");
}

#[test]
fn test_full_process_refmap_theilsen() {
    test_with_data_from("test_data_refmap_theilsen");
}

#[test]
fn test_full_process_querymap_leastsquares() {
    test_with_data_from("test_data_querymap_leastsquares");
}

#[test]
fn test_full_process_refmap_leastsquares() {
    test_with_data_from("test_data_refmap_leastsquares");
}