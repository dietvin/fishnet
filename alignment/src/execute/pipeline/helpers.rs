use std::{any::Any, thread::JoinHandle};

use crossbeam::channel::SendError;

/// Logs `message: err` at the `ERROR` level, mirrors it to `stderr`, and
/// terminates the process with exit code `1`.
///
/// # Arguments
/// * `err`     - Any value that implements [`std::error::Error`].
/// * `message` - A prefix that describes the context in which the error occurred.
pub(super) fn catch_error<E: std::error::Error>(err: E, message: &str) -> ! {
    // -> ! tells the compiler that this function never returns
    let error_message = format!("{message}: {err}");
    log::error!("{}", &error_message);
    eprintln!("{}", error_message);
    std::process::exit(1);
}

/// Unwraps the result of [`std::thread::Builder::spawn`], exiting the process
/// if the OS refused to create the thread.
///
/// On success the [`JoinHandle`] is returned so the caller can join the thread
/// later. On failure the error is logged and printed to `stderr` before calling
/// [`std::process::exit(1)`].
///
/// # Arguments
/// * `res` - The `Result` returned by `thread::Builder::spawn`.
pub(super) fn catch_thread_spawn_error(res: Result<JoinHandle<()>, std::io::Error>) -> JoinHandle<()> {
    match res {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to spawn thread: {e}");
            eprintln!("Failed to spawn thread: {e}");
            std::process::exit(1);
        }
    }
}

/// Handles the result of [`JoinHandle::join`], exiting the process if the
/// thread panicked.
///
/// A successful join is logged at `DEBUG` level. A failed join (i.e. the
/// thread panicked) is logged at `ERROR` level, printed to `stderr`, and
/// causes the process to exit with code `1`.
///
/// # Arguments
/// * `res`  - The `Result` returned by `JoinHandle::join`.
/// * `name` - A display name for the thread used in log and error messages.
pub(super) fn catch_thread_join_error(res: Result<(), Box<dyn Any + Send>>, name: &str) {
    match res {
        Ok(_) => {
            log::debug!("Thread {} finished successfully", name);
        }
        Err(e) => {
            log::error!("Failed to join thread {}: {:?}", name, e);
            eprintln!("Failed to join thread {}: {:?}", name, e);
            std::process::exit(1);
        }
    }
}

/// Checks whether a channel [`Sender::send`] succeeded, exiting the process
/// if the receiver has been dropped (i.e. the channel is disconnected).
///
/// A [`SendError`] indicates that the receiving end of the channel has closed,
/// which is a fatal pipeline condition. The error is logged at `ERROR` level
/// and the process exits with code `1`.
///
/// # Arguments
/// * `res` - The `Result` returned by `Sender::send`.
pub(super) fn catch_channel_error<T>(res: Result<(), SendError<T>>) {
    if let Err(e) = res {
        log::error!("Failed to send data in queue: {e}");
        eprintln!("Failed to send data in queue: {e}");
        std::process::exit(1);
    }
}