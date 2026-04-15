use crossbeam::channel::Sender;
use pod5_reader_api::{dataset::Pod5Dataset, read::Pod5Read};

use crate::{
    bam::{file::BamFileLazy, read::BamRead}, execute::pipeline::helpers::{catch_channel_error, catch_error},
};


/// Drives the data-ingestion stage of the pipeline.
///
/// Iterates over every POD5 file in `pod5_dataset`, and for each raw signal
/// read attempts to look up the matching BAM alignment record by read ID.
/// Successfully paired `(Pod5Read, BamRead, read_id)` tuples are sent to the
/// worker threads via `data_sender`.
///
/// For every read processed a `bool` is sent on `progress_sender` 
/// (`true` = data forwarded, `false` = skipped due to an error) so that the
/// progress thread can update its counters.
///
/// This function runs on its own dedicated thread and returns (dropping
/// `data_sender`) once all POD5 files have been exhausted, which signals
/// downstream worker threads to flush and finish.
///
/// # Arguments
/// * `pod5_dataset`    - The indexed collection of POD5 files to iterate over.
/// * `bam_file`        - The BAM file to query for alignment records.
/// * `data_sender`     - Channel used to pass paired reads to worker threads.
/// * `progress_sender` - Channel used to report per-read success/failure to
///                       the progress thread.
///
/// # Errors / Exits
/// Fatal errors (e.g. failing to open a POD5 file iterator) call
/// [`catch_error`] and exit the process. Recoverable per-read errors (e.g. a
/// corrupted read or a missing BAM entry) are logged and skipped.
pub(super) fn producer_pipeline(
    mut pod5_dataset: Pod5Dataset,
    mut bam_file: BamFileLazy,
    data_sender: Sender<(Pod5Read, BamRead, String)>,
    progress_sender: Sender<bool>
) {
    log::info!("Starting producer thread");

    for pod5_file in pod5_dataset.iter_files_mut() {
        let pod5_read_iter = match pod5_file.iter_reads() {
            Ok(v) => v,
            Err(e) => catch_error(e, &format!(
                "Failed to produce iterator for pod5 file '{}'",
                pod5_file.path().display()
            ))
        };

        for pod5_read_res in pod5_read_iter {
            let pod5_read = match pod5_read_res {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Failed to load pod5 read: {e}");
                    catch_channel_error(progress_sender.send(false));
                    continue;
                }
            };

            let read_id = pod5_read.read_id_string();

            let bam_read = match bam_file.get(&read_id) {
                Ok(v) => v,
                Err(e) => {
                    log::error!(
                        "Failed to retrieve bam read with ID {}: {}",
                        read_id, e
                    );
                    catch_channel_error(progress_sender.send(false));
                    continue;
                }
            };

            catch_channel_error(data_sender.send((pod5_read, bam_read, read_id)));
        }
    }

    drop(data_sender);
}