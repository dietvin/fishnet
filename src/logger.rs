use std::path::PathBuf;

use log::LevelFilter;
use log4rs::{append::file::FileAppender, config::{Appender, Logger, Root}, encode::pattern::PatternEncoder, Config};


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
pub fn setup_logger(
    path: &PathBuf, 
    default_level: LevelFilter,
    module_filters: Vec<(&str, LevelFilter)>,
    include_function_name: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let pattern = if include_function_name {
        "{d(%Y-%m-%d %H:%M:%S)}\t{l}\t{M}:{L}\t{m}\n"
    } else {
        "{d(%Y-%m-%d %H:%M:%S)}\t{l}\t{m}\n"
    };
    
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build(path)?;

    let mut config_builder = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)));

    let mut info_string = String::new();

    for (module, level) in module_filters {
        info_string.push_str(&format!("{}: {},", module, level));

        let logger = Logger::builder()
            .appender("logfile")
            .additive(false)
            .build(module, level);

        config_builder = config_builder.logger(logger)
    }

    let config = config_builder
        .build(Root::builder().appender("logfile").build(default_level))?;

    log4rs::init_config(config)?;
    log::info!("Logger initialized. Writing to file: '{}'. Logging level(s):", path.display());
    log::info!("Logging level root: {}, Module logging levels: {}", default_level, info_string);
    Ok(())
}

/// Retrieves the first and last n elements from a given vector as a String.
/// 
/// This function retrieves the first and last n elements from a vector as 
/// a String representation. The string is intended to be used for debug
/// messages. If the vector is short enough, its entirety gets represented
/// in the string.
/// 
/// # Arguments
///
/// * `vec` - The vector to log
/// * `n` - The number of elements to show from the beginning and end
/// 
/// # Returns
/// 
/// String containing the vector start and end.
pub fn get_log_vector_sample<T: std::fmt::Debug>(vec: &[T], n: usize) -> String {
    if vec.len() <= n * 2 {
        // If vector is small enough, log the whole thing
        format!("{:?}", vec)
    } else {
        // Create a representation showing first n and last n elements
        let first = &vec[..n];
        let last = &vec[vec.len() - n..];
        
        // Format the first and last parts together
        format!("{:?} ... {:?}", first, last)
    }
}