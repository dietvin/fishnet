use log::LevelFilter;
use log4rs::{append::file::FileAppender, config::{Appender, Root}, encode::pattern::PatternEncoder, Config};


/// Sets up a file-based logging system using log4rs.
///
/// This function configures a logger that writes to a file specified by `path` with the
/// provided logging level filter. The log format includes timestamp, module, level, and message.
///
/// # Arguments
///
/// * `path` - The file path where logs will be written
/// * `level_filter` - The minimum log level to capture (e.g., `LevelFilter::Debug`)
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>` - Success or an error if logger setup fails
pub fn setup_logger(path: &str, level_filter: LevelFilter) -> Result<(), Box<dyn std::error::Error>> {
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} {M} : {l} - {m}\n")))
        .build(path)?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(level_filter))?;

    log4rs::init_config(config)?;

    log::info!("Logger initialized. Writing to file: {}", path);
    Ok(())
}

/// Logs a sample of a vector showing both the first and last elements.
///
/// This function logs a debug message containing either the complete vector (if it's
/// small enough) or a sample showing the first and last `n` elements.
///
/// # Arguments
///
/// * `vec` - The vector to log
/// * `n` - The number of elements to show from the beginning and end
/// * `name` - Optional name to identify the vector in the log (defaults to "Vector")
pub fn log_vector_sample<T: std::fmt::Debug>(
    vec: &[T], 
    n: usize,
    name: Option<&str>
) {
    let vec_name = name.unwrap_or("Vector");
    
    if vec.len() <= n * 2 {
        // If vector is small enough, log the whole thing
        log::debug!("{} (complete, {} items): {:?}", vec_name, vec.len(), vec);
    } else {
        // Create a representation showing first n and last n elements
        let first = &vec[..n];
        let last = &vec[vec.len() - n..];
        
        // Format the first and last parts together
        log::debug!(
            "{} ({} items): {:?} ... {:?}", 
            vec_name,
            vec.len(), 
            first, 
            last
        );
    }
}