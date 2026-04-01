use std::time::Duration;

use console::style;
use crossbeam::channel::Receiver;
use indicatif::{ProgressBar, ProgressStyle};


/// Displays a progress bar for the duration of the pipeline run.
///
/// Receives per-read outcome signals from the producer and worker threads via
/// `progress_receiver`. Each `true` value indicates a successfully processed
/// read; each `false` indicates a read that was skipped due to an error.
/// Running totals for both categories are shown in the spinner message and
/// updated every 100 reads to avoid excessive redraw overhead.
///
/// When `progress_receiver` is closed (i.e. all senders have been dropped),
/// the function performs a final update, transitions the spinner to a
/// completion message showing overall counts in green/red, and returns.
///
/// This function is designed to run on its own dedicated thread spawned by
/// [`start_pipeline`].
///
/// # Arguments
/// * `progress_receiver` - Channel over which producer and worker threads send
///                         per-read success (`true`) or failure (`false`) flags.
pub(super) fn progress_pipeline(
    progress_receiver: Receiver<bool>
) {
    let mut counter = 0;
    let mut n_success = 0;
    let mut n_failed = 0;

    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] Processed {pos} reads | {msg}")                    
            .unwrap()            
    );
    progress_bar.enable_steady_tick(Duration::from_millis(100));

    for is_success in progress_receiver {
        if is_success {
            n_success += 1;
        } else {
            n_failed += 1;
        }

        counter += 1;
        if counter % 100 == 0 {
            progress_bar.set_message(format!("{} ✓ | {} ✗", n_success, n_failed));
            progress_bar.inc(100);
        }
    }

    progress_bar.set_position(counter);
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} [{elapsed_precise}] {msg}")
            .unwrap()       
    );

    progress_bar.finish_with_message(format!(
        "{} | {} | {}",
        style(format!("Finished processing {} reads", progress_bar.position())).green(),
        style(format!("{} ✓ ", n_success)).green(),
        style(format!("{} ✗ ", n_failed)).red()
    ));
}