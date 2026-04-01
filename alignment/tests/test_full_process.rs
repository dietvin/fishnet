use alignment::bam::file::BamFileLazy;
use alignment::bam::read::BamRead;
use alignment::core::alignment::aligned_read::{QueryAligned, RefAligned};
use alignment::core::alignment::{AlignBoth, AlignQueryOnly, AlignmentMode};
use alignment::core::refinement::dp::forward_step::dwell_penalty::DwellPenalty;
use alignment::core::refinement::dp::forward_step::viterbi::Viterbi;
use alignment::core::refinement::rescaling::theil_sen::TheilSen;
use alignment::core::refinement::rough_rescaling::least_squares::RoughLeastSquares;
use alignment::core::refinement::rough_rescaling::skip::SkipRoughRescaling;
use alignment::core::refinement::rough_rescaling::theil_sen::RoughTheilSen;
use alignment::core::refinement::{RefineQueryToSignal, RefineRefToSignal, RefinementMode};
use alignment::core::refinement::dp::forward_step::RefinementAlgo;
use alignment::core::refinement::rescaling::RescaleAlgo;
use alignment::core::refinement::rough_rescaling::RoughRescaleAlgo;
use alignment::output::record::{QueryToSignalResult, RefToSignalResult};
use kmer_table::kmer_table::KmerTable;
use pod5_reader_api::file::Pod5File;
use pod5_reader_api::read::Pod5Read;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fs::File;
use std::io::BufReader;
use walkdir::WalkDir;
use std::path::PathBuf;


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


#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct SdParams {
    target: f32,
    limit: f32,
    weight: f32
}


fn load_json(path: &PathBuf) -> JsonData {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: JsonData = serde_json::from_reader(reader).unwrap();

    data
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


fn test_directory(dirname: &str) {
    let dir = format!("tests/{}/full_process", dirname);

    let path = PathBuf::from("../example_data/remora_example/can_reads.pod5");
    let mut pod5_file = Pod5File::new(&path).unwrap();

    let path = "../example_data/remora_example/can_mappings.bam";
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
        let json_data = load_json(&file);

        let read_id = json_data.read_id.clone();

        let bam_read = bam_file.get(&read_id).unwrap();
        let read_uuid = Uuid::try_from(read_id).unwrap();
        let pod5_read = pod5_file.get(&read_uuid).unwrap();

        let kmer_table = KmerTable::from_file(
            &PathBuf::from("../example_data/remora_example/levels.txt"),
            false
        ).unwrap();

        choose_parameters(
            path_str,
            json_data,
            &pod5_read,
            &bam_read,
            &kmer_table
        );
    }
}

fn choose_parameters(
    path_str: &str,
    json_data: JsonData,
    pod5_read: &Pod5Read,
    bam_read: &BamRead,
    kmer_table: &KmerTable
) {
    let n_refinement_iters = match json_data.scale_iters {
        -1 => 0,
        n => n as usize,
    };

    let rescale_algo = TheilSen::new(
        0.1,
        0.9,
        0.2,
        10,
        10,
        1000000000,
    );

    let rough_quantiles = vec![
        0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5,
        0.55, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95,
    ];

    macro_rules! run {
        ($rough_algo:expr) => {
            match json_data.algo.as_str() {
                "dwell_penalty" => {
                    let refinement_algo = DwellPenalty::new(
                        json_data.sd_params.target,
                        json_data.sd_params.limit,
                        json_data.sd_params.weight,
                    );
                    run_alignment!(
                        path_str, json_data, pod5_read, bam_read, kmer_table,
                        n_refinement_iters, $rough_algo, rescale_algo, refinement_algo
                    )
                }
                "Viterbi" => {
                    run_alignment!(
                        path_str, json_data, pod5_read, bam_read, kmer_table,
                        n_refinement_iters, $rough_algo, rescale_algo, Viterbi
                    )
                }
                _ => unreachable!(),
            }
        };
    }

    macro_rules! run_alignment {
        ($path:expr, $json:expr, $pod5:expr, $bam:expr, $kmer:expr,
         $iters:expr, $rough:expr, $rescale:expr, $algo:expr) => {{
            if $json.reference_mapping {
                let refinement_mode = RefineRefToSignal::new(
                    $iters, $json.half_bandwidth, true, 2, $rough, $rescale, $algo
                );
                test_ref_alignment($path, $pod5, $bam, $kmer,
                    AlignBoth::new(false), refinement_mode, $json);
            } else {
                let refinement_mode = RefineQueryToSignal::new(
                    $iters, $json.half_bandwidth, true, 2, $rough, $rescale, $algo
                );
                test_query_alignment($path, $pod5, $bam, $kmer,
                    AlignQueryOnly::new(false), refinement_mode, $json);
            }
        }};
    }

    match (json_data.do_rough_rescale, json_data.rough_rescale_method.as_str()) {
        (true, "least_squares") => run!(RoughLeastSquares::new(rough_quantiles, 10, true)),
        (true, "theil_sen")     => run!(RoughTheilSen::new(rough_quantiles, 10, true)),
        (false, _)              => run!(SkipRoughRescaling::new(vec![], 0, true)),
        _                       => unreachable!(),
    }
}


fn test_query_alignment<S, T, U, R>(
    path_str: &str,
    pod5_read: &Pod5Read,
    bam_read: &BamRead,
    kmer_table: &KmerTable,
    alignment_mode: AlignQueryOnly,
    refinement_mode: R,
    json_data: JsonData
)
where
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo,
    R: RefinementMode<S, T, U, Input = QueryAligned, Output = QueryToSignalResult>
{
    let aligned_read = alignment_mode.align(pod5_read, bam_read).unwrap();

    let offset = aligned_read.base.signal_offset();

    let query_to_signal = &aligned_read.query_to_signal;
    assert_eq!(*query_to_signal, json_data.unrefined_map);

    let refined_data = refinement_mode.refine(
        aligned_read,
        kmer_table,
        pod5_read,
        bam_read
    ).unwrap();

    // The Remora-generated maps do not account for the offset adjustments,
    // so the offset needs to be removed again for testing
    let mut query_to_sig = refined_data.query_to_sig;
    query_to_sig.iter_mut().for_each(|el| *el -= offset);
    
    let (passes_test, pct_mismatch, n_differences) = test_vectors_95_percent_equal(
        &query_to_sig,
        &json_data.refined_map
    );
    assert!(
        passes_test,
        "{} | Pct mismatch: {} ({} different positions)",
        path_str,
        pct_mismatch,
        n_differences
    );
}

fn test_ref_alignment<S, T, U, R>(
    path_str: &str,
    pod5_read: &Pod5Read,
    bam_read: &BamRead,
    kmer_table: &KmerTable,
    alignment_mode: AlignBoth,
    refinement_mode: R,
    json_data: JsonData
)
where
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo,
    R: RefinementMode<S, T, U, Input = RefAligned, Output = RefToSignalResult>
{
    let aligned_read = alignment_mode.align(pod5_read, bam_read).unwrap();

    let offset = aligned_read.base.signal_offset();

    let ref_to_signal = &aligned_read.ref_to_signal;
    assert_eq!(*ref_to_signal, json_data.unrefined_map);

    let refined_data = refinement_mode.refine(
        aligned_read,
        kmer_table,
        pod5_read,
        bam_read
    ).unwrap();

    // The Remora-generated maps do not account for the offset adjustments,
    // so the offset needs to be removed again for testing
    let mut ref_to_sig = refined_data.ref_to_sig;
    ref_to_sig.iter_mut().for_each(|el| *el -= offset);

    let (passes_test, pct_mismatch, n_differences) = test_vectors_95_percent_equal(
        &ref_to_sig,
        &json_data.refined_map
    );
    assert!(
        passes_test,
        "{} | Pct mismatch: {} ({} different positions)",
        path_str,
        pct_mismatch,
        n_differences
    );
}


#[test]
fn test_full_process_querymap_theilsen() {
    test_directory("test_data_querymap_theilsen");
}

#[test]
fn test_full_process_refmap_theilsen() {
    test_directory("test_data_refmap_theilsen");
}

#[test]
fn test_full_process_querymap_leastsquares() {
    test_directory("test_data_querymap_leastsquares");
}

#[test]
fn test_full_process_refmap_leastsquares() {
    test_directory("test_data_refmap_leastsquares");
}