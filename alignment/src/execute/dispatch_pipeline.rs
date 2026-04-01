use helper::io::OutputFormat;

use crate::{
    core::{alignment::{AlignBoth, AlignQueryOnly, AlignmentMode},
        refinement::{RefineBoth, RefineQueryToSignal, RefineRefToSignal, RefinementMode, dp::forward_step::{RefinementAlgo, dwell_penalty::DwellPenalty, viterbi::Viterbi}, rescaling::{RescaleAlgo, least_squares::LeastSquares, theil_sen::TheilSen}, rough_rescaling::{
            RoughRescaleAlgo,
            least_squares::RoughLeastSquares,
            skip::SkipRoughRescaling,
            theil_sen::RoughTheilSen}
        }
    }, 
    execute::{config::{
        AlignmentType, Config, OutputLevel, RefineAlgoOptions, RescaleAlgoOptions, RoughRescaleAlgoOptions}, pipeline::start_pipeline}, output::{buffer::{jsonl::JsonlBuffer, parquet::ParquetBuffer}, record::IntoOutputRecord, schema::{BothBasic, BothWithSeq, BothWithSeqAndSig, OutputSchema, QueryBasic, QueryWithSeq, QueryWithSeqAndSig, RefBasic, RefWithSeq, RefWithSeqAndSig}, writer::{jsonl::JsonlWriter, parquet::ParquetWriter}}
};


/// Convenience macro that expands to a fully-qualified [`start_pipeline`] call
/// with all eight type parameters in scope.
///
/// The macro exists purely to avoid repeating the long turbofish syntax at
/// every dispatch leaf. It takes the eight type parameters as the first
/// arguments, followed by the four runtime values required by
/// [`start_pipeline`].
///
/// # Parameters (positional)
/// 1. `$align_mode`            - [`AlignmentMode`] implementation type.
/// 2. `$rough_rescale_algo`    - [`RoughRescaleAlgo`] implementation type.
/// 3. `$rescale_algo`          - [`RescaleAlgo`] implementation type.
/// 4. `$refine_algo`           - [`RefinementAlgo`] implementation type.
/// 5. `$refine_mode`           - [`RefinementMode`] implementation type.
/// 6. `$output_schema`         - [`OutputSchema`] implementation type.
/// 7. `$output_buffer_ty`      - [`Buffer`] implementation type.
/// 8. `$output_writer_ty`      - [`Writer`] implementation type.
/// 9. `$config`                - Owned [`Config`] value.
/// 10. `$alignment_mode`       - Concrete alignment mode instance.
/// 11. `$refinement_mode`      - Concrete refinement mode instance.
/// 12. `$output_buffer`        - Concrete buffer instance.
/// 13. `$output_writer`        - Concrete writer instance.
macro_rules! dispatch_pipeline {
    (
        $align_mode:ty,
        $rough_rescale_algo:ty, 
        $rescale_algo:ty,
        $refine_algo:ty,
        $refine_mode:ty,
        $output_schema:ty,
        $output_buffer_ty:ty,
        $output_writer_ty:ty,
        $config:expr,
        $alignment_mode:expr,
        $refinement_mode:expr,
        $output_buffer:expr,
        $output_writer:expr
    ) => {{
        start_pipeline::<
            $align_mode,
            $rough_rescale_algo,
            $rescale_algo,
            $refine_algo,
            $refine_mode,
            $output_schema,
            $output_buffer_ty,
            $output_writer_ty
        >(
            $config,
            $alignment_mode,
            $refinement_mode,
            $output_buffer,
            $output_writer
        );
    }};
}


/// Top-level dispatch entry point called from [`execute`].
///
/// Begins the chain of runtime-to-compile-time dispatch by resolving the rough
/// rescaling algorithm first. See [`dispatch_rough_rescale`] for the next step.
pub(super) fn dispatch(config: Config) {
    dispatch_rough_rescale(config);
}


/// Resolves the rough rescaling algorithm and passes the concrete type
/// downstream.
///
/// Matches on [`Config::rough_rescale_config`]:
/// * `Some(TheilSen)`     → constructs [`RoughTheilSen`] and continues.
/// * `Some(LeastSquares)` → constructs [`RoughLeastSquares`] and continues.
/// * `None`               → constructs a zero-cost [`SkipRoughRescaling`]
///                          sentinel that satisfies the trait bound without
///                          performing any computation.
///
/// After this function the rough rescaling type parameter `S` is fixed and
/// carried through the remaining dispatch chain as a monomorphised generic.
fn dispatch_rough_rescale(config: Config) {
    match &config.rough_rescale_config {
        Some(rough_rescale_config) => {
            match rough_rescale_config.algo {
                RoughRescaleAlgoOptions::TheilSen => {
                    let rough_rescale_algo = RoughTheilSen::new(
                        rough_rescale_config.quantiles.clone(),
                        rough_rescale_config.clip_bases,
                        rough_rescale_config.use_base_center
                    );
                    dispatch_rescale::<RoughTheilSen>(config, rough_rescale_algo);
                }
                RoughRescaleAlgoOptions::LeastSquares => {
                    let rough_rescale_algo = RoughLeastSquares::new(
                        rough_rescale_config.quantiles.clone(),
                        rough_rescale_config.clip_bases,
                        rough_rescale_config.use_base_center
                    );
                    dispatch_rescale::<RoughLeastSquares>(config, rough_rescale_algo);
                }
            }
        }
        None => {
            let rough_rescale_algo = SkipRoughRescaling::new(vec![], 0, true);
            dispatch_rescale::<SkipRoughRescaling>(config, rough_rescale_algo);
        }
    }
}


/// Resolves the fine rescaling algorithm, fixing type parameter `T`.
///
/// Matches on [`Config::rescale_algo`] and constructs the appropriate
/// [`RescaleAlgo`] implementation ([`TheilSen`] or [`LeastSquares`]) from the
/// stored parameters, then forwards to [`dispatch_refinement_algo`].
///
/// # Type Parameters
/// * `S` - Already-resolved rough rescaling type from [`dispatch_rough_rescale`].
fn dispatch_rescale<S>(
    config: Config,
    rough_rescale_algo: S
) 
where
    S: RoughRescaleAlgo + 'static
{
    match config.rescale_algo {
        RescaleAlgoOptions::TheilSen { 
            dwell_filter_lower_percentile,
            dwell_filter_upper_percentile,
            min_abs_level,
            n_bases_truncate,
            min_num_filtered_levels,
            max_points
        } => {
            let rescale_algo = TheilSen::new(
                dwell_filter_lower_percentile,
                dwell_filter_upper_percentile,
                min_abs_level,
                n_bases_truncate,
                min_num_filtered_levels,
                max_points
            );
            dispatch_refinement_algo::<S, TheilSen>(
                config,
                rough_rescale_algo,
                rescale_algo
            );
        }
        RescaleAlgoOptions::LeastSquares {
            dwell_filter_lower_percentile,
            dwell_filter_upper_percentile,
            min_abs_level,
            n_bases_truncate,
            min_num_filtered_levels
        } => {
            let rescale_algo = LeastSquares::new(
                dwell_filter_lower_percentile,
                dwell_filter_upper_percentile,
                min_abs_level,
                n_bases_truncate,
                min_num_filtered_levels
            );
            dispatch_refinement_algo::<S, LeastSquares>(
                config,
                rough_rescale_algo,
                rescale_algo
            );
        }
    }
}


/// Resolves the DP refinement algorithm, fixing type parameter `U`.
///
/// Matches on [`Config::refine_algo`] and constructs the appropriate
/// [`RefinementAlgo`] implementation ([`Viterbi`] or [`DwellPenalty`]), then
/// forwards to [`dispatch_alignment_and_refinement`].
///
/// # Type Parameters
/// * `S` - Already-resolved rough rescaling type.
/// * `T` - Already-resolved fine rescaling type.
fn dispatch_refinement_algo<S, T>(
    config: Config,
    rough_rescale_algo: S,
    rescale_algo: T
)
where
    S: RoughRescaleAlgo + 'static,
    T: RescaleAlgo + 'static
{
    match config.refine_algo {
        RefineAlgoOptions::Viterbi => {
            let refinement_algo = Viterbi;
            dispatch_alignment_and_refinement::<S, T, Viterbi>(
                config,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
        }
        RefineAlgoOptions::DwellPenalty {
            target,
            limit,
            weight
        } => {
            let refinement_algo = DwellPenalty::new(target, limit, weight);
            dispatch_alignment_and_refinement::<S, T, DwellPenalty>(
                config,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
        }
    }
}


/// Resolves the alignment mode and refinement mode types, fixing the
/// remaining compile-time generics before output dispatch.
///
/// Matches on the Cartesian product of [`Config::alignment_type`] ×
/// [`Config::output_config.level`] (nine combinations in total) and
/// constructs the appropriate concrete [`AlignmentMode`] and
/// [`RefinementMode`] instances for each:
///
/// | `alignment_type` | `output_level`  | Alignment type     | Refinement type              | Output schema            |
/// |------------------|-----------------|--------------------|------------------------------|--------------------------|
/// | `Query`          | `Minimal`       | [`AlignQueryOnly`] | [`RefineQueryToSignal`]      | [`QueryBasic`]           |
/// | `Query`          | `WithSeq`       | [`AlignQueryOnly`] | [`RefineQueryToSignal`]      | [`QueryWithSeq`]         |
/// | `Query`          | `WithSeqAndSig` | [`AlignQueryOnly`] | [`RefineQueryToSignal`]      | [`QueryWithSeqAndSig`]   |
/// | `Reference`      | `Minimal`       | [`AlignBoth`]      | [`RefineRefToSignal`]        | [`RefBasic`]             |
/// | `Reference`      | `WithSeq`       | [`AlignBoth`]      | [`RefineRefToSignal`]        | [`RefWithSeq`]           |
/// | `Reference`      | `WithSeqAndSig` | [`AlignBoth`]      | [`RefineRefToSignal`]        | [`RefWithSeqAndSig`]     |
/// | `Both`           | `Minimal`       | [`AlignBoth`]      | [`RefineBoth`]               | [`BothBasic`]            |
/// | `Both`           | `WithSeq`       | [`AlignBoth`]      | [`RefineBoth`]               | [`BothWithSeq`]          |
/// | `Both`           | `WithSeqAndSig` | [`AlignBoth`]      | [`RefineBoth`]               | [`BothWithSeqAndSig`]    |
///
/// Note that `Reference` and `Both` alignment types both use [`AlignBoth`]
/// because the reference sequence can only be obtained alongside the query
/// alignment; the refinement mode then selects which result is forwarded.
///
/// After this function all generic type parameters are resolved and execution
/// is passed to [`dispatch_output`].
///
/// # Type Parameters
/// * `S` - Already-resolved rough rescaling type.
/// * `T` - Already-resolved fine rescaling type.
/// * `U` - Already-resolved DP refinement algorithm type.
fn dispatch_alignment_and_refinement<S, T, U>(
    config: Config,
    rough_rescale_algo: S,
    rescale_algo: T,
    refinement_algo: U
)
where
    S: RoughRescaleAlgo + 'static,
    T: RescaleAlgo + 'static,
    U: RefinementAlgo + 'static
{
    match (&config.alignment_type, &config.output_config.level) {
        (AlignmentType::Query, OutputLevel::Minimal) => {
            let alignment_mode = AlignQueryOnly::new(config.is_drna);

            let refinement_mode = RefineQueryToSignal::new(
                config.refine_iters,
                config.band_config.half_bandwidth,
                true,
                config.band_config.min_step,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );

            dispatch_output::<AlignQueryOnly, S, T, U, RefineQueryToSignal<S, T, U>, QueryBasic>(
                config,
                alignment_mode,
                refinement_mode
            );
        }
        (AlignmentType::Reference, OutputLevel::Minimal) => {
            let alignment_mode = AlignBoth::new(config.is_drna);

            let refinement_mode = RefineRefToSignal::new(
                config.refine_iters,
                config.band_config.half_bandwidth,
                true,
                config.band_config.min_step,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
            
            dispatch_output::<AlignBoth, S, T, U, RefineRefToSignal<S, T, U>, RefBasic>(
                config,
                alignment_mode,
                refinement_mode
            );
        }
        (AlignmentType::Both, OutputLevel::Minimal) => {
            let alignment_mode = AlignBoth::new(config.is_drna);

            let refinement_mode = RefineBoth::new(
                config.refine_iters,
                config.band_config.half_bandwidth,
                true,
                config.band_config.min_step,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
            
            dispatch_output::<AlignBoth, S, T, U, RefineBoth<S, T, U>, BothBasic>(
                config,
                alignment_mode,
                refinement_mode
            );
        }
        (AlignmentType::Query, OutputLevel::WithSeq) => {
            let alignment_mode = AlignQueryOnly::new(config.is_drna);

            let refinement_mode = RefineQueryToSignal::new(
                config.refine_iters,
                config.band_config.half_bandwidth,
                true,
                config.band_config.min_step,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
            
            dispatch_output::<AlignQueryOnly, S, T, U, RefineQueryToSignal<S, T, U>, QueryWithSeq>(
                config,
                alignment_mode,
                refinement_mode
            );
        }
        (AlignmentType::Reference, OutputLevel::WithSeq) => {
            let alignment_mode = AlignBoth::new(config.is_drna);

            let refinement_mode = RefineRefToSignal::new(
                config.refine_iters,
                config.band_config.half_bandwidth,
                true,
                config.band_config.min_step,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
            
            dispatch_output::<AlignBoth, S, T, U, RefineRefToSignal<S, T, U>, RefWithSeq>(
                config,
                alignment_mode,
                refinement_mode
            );
        }
        (AlignmentType::Both, OutputLevel::WithSeq) => {
            let alignment_mode = AlignBoth::new(config.is_drna);

            let refinement_mode = RefineBoth::new(
                config.refine_iters,
                config.band_config.half_bandwidth,
                true,
                config.band_config.min_step,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
            
            dispatch_output::<AlignBoth, S, T, U, RefineBoth<S, T, U>, BothWithSeq>(
                config,
                alignment_mode,
                refinement_mode
            );
        }
        (AlignmentType::Query, OutputLevel::WithSeqAndSig) => {
            let alignment_mode = AlignQueryOnly::new(config.is_drna);

            let refinement_mode = RefineQueryToSignal::new(
                config.refine_iters,
                config.band_config.half_bandwidth,
                true,
                config.band_config.min_step,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
            
            dispatch_output::<AlignQueryOnly, S, T, U, RefineQueryToSignal<S, T, U>, QueryWithSeqAndSig>(
                config,
                alignment_mode,
                refinement_mode
            );
        }
        (AlignmentType::Reference, OutputLevel::WithSeqAndSig) => {
            let alignment_mode = AlignBoth::new(config.is_drna);

            let refinement_mode = RefineRefToSignal::new(
                config.refine_iters,
                config.band_config.half_bandwidth,
                true,
                config.band_config.min_step,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
            
            dispatch_output::<AlignBoth, S, T, U, RefineRefToSignal<S, T, U>, RefWithSeqAndSig>(
                config,
                alignment_mode,
                refinement_mode
            );
        }
        (AlignmentType::Both, OutputLevel::WithSeqAndSig) => {
            let alignment_mode = AlignBoth::new(config.is_drna);

            let refinement_mode = RefineBoth::new(
                config.refine_iters,
                config.band_config.half_bandwidth,
                true,
                config.band_config.min_step,
                rough_rescale_algo,
                rescale_algo,
                refinement_algo
            );
            
            dispatch_output::<AlignBoth, S, T, U, RefineBoth<S, T, U>, BothWithSeqAndSig>(
                config,
                alignment_mode,
                refinement_mode
            );
        }
    }
}


/// Resolves the output buffer and writer types, completing the dispatch chain.
///
/// Matches on [`Config::output_config.format`] and constructs the appropriate
/// concrete [`Buffer`] and [`Writer`] pair:
///
/// * [`OutputFormat::Json`]    -> [`JsonlBuffer`] + [`JsonlWriter`]
/// * [`OutputFormat::Parquet`] -> [`ParquetBuffer`] + [`ParquetWriter`]
///
/// Writer initialisation failures (e.g. a permissions error on the output
/// path) are logged at `ERROR` level and cause the process to exit with code
/// `1` before the pipeline is started.
///
/// Once both objects are constructed this function calls the
/// [`dispatch_pipeline!`] macro which expands to the fully-typed
/// [`start_pipeline`] invocation, launching all threads.
///
/// # Type Parameters
/// * `A`  – Alignment mode.
/// * `S`  – Rough rescaling algorithm.
/// * `T`  – Fine rescaling algorithm.
/// * `U`  – DP refinement algorithm.
/// * `R`  – Refinement mode wiring `S`, `T`, `U` together.
/// * `OS` – Output schema.
fn dispatch_output<A, S, T, U, R, OS>(
    config: Config,
    alignment_mode: A,
    refinement_mode: R
)
where
    A: AlignmentMode + 'static,
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo,
    R: RefinementMode<S, T, U, Input = A::Output> + 'static,
    R::Output: IntoOutputRecord<OS>,
    OS: OutputSchema
{
    match config.output_config.format {
        OutputFormat::Json => {
            let buffer = JsonlBuffer::new(
                config.output_config.batch_size_bytes
            );

            let writer = match JsonlWriter::new(
                &config.output_config.path,
                config.output_config.force_overwrite
            ) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to intialize output file: {e}");
                    std::process::exit(1);
                }
            };

            dispatch_pipeline!(
                A, S, T, U, R, OS, JsonlBuffer, JsonlWriter,
                config,
                alignment_mode,
                refinement_mode,
                buffer,
                writer
            );
        }
        OutputFormat::Parquet => {
            let buffer = ParquetBuffer::new(
                config.output_config.batch_size_bytes
            );

            let writer = match ParquetWriter::new::<OS>(
                &config.output_config.path,
                config.output_config.force_overwrite
            ) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to intialize output file: {e}");
                    std::process::exit(1);
                }
            };

            dispatch_pipeline!(
                A, S, T, U, R, OS, ParquetBuffer, ParquetWriter,
                config,
                alignment_mode,
                refinement_mode,
                buffer,
                writer
            );
        }
        _ => unreachable!()
    }
}