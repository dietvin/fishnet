use rust_htslib::bam::record::{Aux, Record};
use super::super::error::loader_errors::{bam_errors::BamReadError, pod5_errors::Pod5FileError};

// ########################################################################################################################
//                                             Helper functions for BamRead
// ########################################################################################################################

/// Handles a tag fetch result by either returning the value or substituting a default value
/// for certain error types.
/// 
/// This is done because some tags may not be set (e.g. 'sp' (default=0) or 'ts' (default=0)).
///
/// # Arguments
///
/// * `get_tag_result` - The result of attempting to fetch a tag from a BAM record
/// * `default_value` - The default value to use if an HTSLib error occurs
///
/// # Returns
///
/// * `Ok(Some(value))` - If the tag was retrieved successfully or an HTSLib error occurred
/// * `Err(e)` - If a non-HTSLib error occurred
pub fn unpack_tag<V>(get_tag_result: Result<V, BamReadError>, default_value: Option<V>) -> Result<Option<V>, BamReadError> {
    match get_tag_result {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            match e {
                BamReadError::HTSLibError(_) => Ok(default_value),
                _ => Err(e)
            }
        }
    }
}

/// Retrieves an integer array tag from a BAM record.
///
/// This function handles different integer array types (i8, i16, i32) and
/// converts them to a Vec<i16>.
///
/// # Arguments
///
/// * `bam_record` - The BAM record to retrieve the tag from
/// * `tag` - The tag name (e.g., "RG", "NM")
///
/// # Returns
///
/// * `Ok(Vec<i16>)` - The integer array if successful
/// * `Err(BamReadError)` - If the tag is missing or has an unexpected type
pub fn get_iarray_tag(bam_record: &Record, tag: &str) -> Result<Vec<i16>, BamReadError> {
    match bam_record.aux(tag.as_bytes()) {
        Ok(Aux::ArrayI8(vec)) => Ok(vec.iter().map(|el| el as i16).collect::<Vec<i16>>()),
        Ok(Aux::ArrayI16(vec)) => Ok(vec.iter().map(|el| el as i16).collect::<Vec<i16>>()),
        Ok(Aux::ArrayI32(vec)) => Ok(vec.iter().map(|el| el as i16).collect::<Vec<i16>>()),
        Err(e) => Err(BamReadError::HTSLibError(e)),
        Ok(other) => handle_unexpected_type(other, tag, "signed int array")
    }
}

/// Retrieves an unsigned integer tag from a BAM record.
///
/// This function handles different unsigned integer types (u8, u16, u32) and
/// converts them to a usize.
///
/// # Arguments
///
/// * `bam_record` - The BAM record to retrieve the tag from
/// * `tag` - The tag name (e.g., "RG", "NM")
///
/// # Returns
///
/// * `Ok(usize)` - The unsigned integer if successful
/// * `Err(BamReadError)` - If the tag is missing or has an unexpected type
pub fn get_uint_tag(bam_record: &Record, tag: &str) -> Result<usize, BamReadError> {
    match bam_record.aux(tag.as_bytes()) {
        Ok(rust_htslib::bam::record::Aux::U8(value)) => Ok(value as usize),
        Ok(rust_htslib::bam::record::Aux::U16(value)) => Ok(value as usize),
        Ok(rust_htslib::bam::record::Aux::U32(value)) => Ok(value as usize),
        Err(e) => Err(BamReadError::HTSLibError(e)),
        Ok(other) => handle_unexpected_type(other, tag, "unsigned int")
    }
}

/// Retrieves a string tag from a BAM record.
///
/// # Arguments
///
/// * `bam_record` - The BAM record to retrieve the tag from
/// * `tag` - The tag name (e.g., "RG", "PG")
///
/// # Returns
///
/// * `Ok(String)` - The string value if successful
/// * `Err(BamReadError)` - If the tag is missing or has an unexpected type
pub fn get_str_tag(bam_record: &Record, tag: &str) -> Result<String, BamReadError> {
    match bam_record.aux(tag.as_bytes()) {
        Ok(rust_htslib::bam::record::Aux::String(value)) => Ok(value.to_string()),
        Err(e) => Err(BamReadError::HTSLibError(e)),
        Ok(other) => handle_unexpected_type(other, tag, "string")
    }
}

/// Retrieves a float tag from a BAM record.
///
/// # Arguments
///
/// * `bam_record` - The BAM record to retrieve the tag from
/// * `tag` - The tag name (e.g., "AS", "XQ")
///
/// # Returns
///
/// * `Ok(f32)` - The float value if successful
/// * `Err(BamReadError)` - If the tag is missing or has an unexpected type
pub fn get_float_tag(bam_record: &Record, tag: &str) -> Result<f32, BamReadError> {
    match bam_record.aux(tag.as_bytes()) {
        Ok(rust_htslib::bam::record::Aux::Float(value)) => Ok(value),
        Err(e) => Err(BamReadError::HTSLibError(e)),
        Ok(other) => handle_unexpected_type(other, tag, "float")
    }
}

/// Helper function to handle unexpected tag types.
///
/// # Arguments
///
/// * `value` - The actual value retrieved from the tag
/// * `tag` - The tag name that was being accessed
/// * `exp_type` - A string describing the expected type
///
/// # Returns
///
/// * `Err(BamReadError::TagUnexpectedTypeError)` - Always returns an error
fn handle_unexpected_type<V>(value: Aux<'_>, tag: &str, exp_type: &str) -> Result<V, BamReadError> {
    let type_name = std::any::type_name_of_val(&value).to_string();
    Err(BamReadError::TagUnexpectedTypeError(
        tag.to_string(), 
        exp_type.to_string(), 
        type_name
    ))
}











// ########################################################################################################################
//                                             Helper functions for Pod5Read
// ########################################################################################################################

/// Reverses a given signal vector
/// 
/// # Arguments
/// * `signal` - Vector of the current measurements
pub fn reverse_signal(signal: &Vec<i16>) -> Vec<i16> {
    signal.iter().rev().map(|el| *el).collect::<Vec<i16>>()
}











// ########################################################################################################################
//                                             Helper functions for Pod5File
// ########################################################################################################################

/// Converts a binary read ID from a Pod5 file into a UTF-8 string
/// 
/// In Pod5 files, read IDs are stored as binary values. This function safely
/// converts these binary values into standard UTF-8 strings that can be used
/// as keys in the HashMap.
/// 
/// # Arguments
/// * `binary_id` - A byte slice containing the binary representation of a read ID
/// 
/// # Returns
/// * `Result<String, Pod5FileError>` - The read ID as a String on success, or an error
/// 
/// # Errors
/// * `Pod5FileError::ColumnDataMissingError` - If the binary_id is None
/// * `Pod5FileError::Utf8Error` - If the binary data cannot be converted to a valid UTF-8 string
use uuid::Uuid;

pub fn read_id_from_binary(binary_id: Option<&[u8]>) -> Result<String, Pod5FileError> {
    let binary_id = binary_id.ok_or(Pod5FileError::ColumnDataMissingError { 
        column: "read_id".to_string(), 
        read_id: "NA".to_string()
    })?;

    let read_id = Uuid::from_slice(binary_id)?.to_string();
    
    Ok(read_id)
}










// ########################################################################################################################
//                                             Helper functions for file handling
// ########################################################################################################################
use std::{path::{PathBuf, Path}, ffi::OsStr, fs};
use walkdir::WalkDir;
use super::super::error::loader_errors::file_handling_errors::{DirHandlingError, FileHandlingError};

pub fn find_files_in_dir(path: &str, file_type: &str, recursive: bool) -> Result<Vec<String>,DirHandlingError> {
    let path_obj = Path::new(path);

    if !path_obj.exists() || !path_obj.is_dir() {
        return Err(
            DirHandlingError::DirectoryNotFound(path.to_string())
        );
    }

    let mut file_paths = Vec::new();

    if recursive {
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok()) {
            let file_path = entry.into_path();

            if valid_type_path(&file_path, file_type) {
                // Maybe include check for non-unicode chars (return error if found)
                file_paths.push(
                    file_path.clone()
                        .to_str()
                        .ok_or(DirHandlingError::PathConvError(file_path))?
                        .to_string()
                    );
            }
        }
    } else {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();
            if valid_type_path(&file_path, file_type) {
                file_paths.push(
                    file_path.clone()
                        .to_str()
                        .ok_or(DirHandlingError::PathConvError(file_path))?
                        .to_string()
                );
            }
        }
    }

    if file_paths.len() > 0 {
        Ok(file_paths)
    } else {
        Err(DirHandlingError::NoFilesFound(file_type.to_string(), path.to_string()))
    }
}

fn valid_type_path(file_path: &PathBuf, extension: &str) -> bool {
    file_path.is_file() && file_path.extension() == Some(OsStr::new(extension))
}

pub fn get_files(file_paths: &Vec<String>, file_type: &str) -> Result<Vec<String>,FileHandlingError> {
    let mut valid_paths = Vec::new();

    for path in file_paths {
        let file_path = Path::new(path);

        if !file_path.exists() || !file_path.is_file() {
            return Err(FileHandlingError::FileNotFound(path.clone()));
        }

        if file_path.extension() != Some(OsStr::new(file_type)) {
            return Err(FileHandlingError::InvalidFileType(
                path.clone(), 
                file_type.to_string()
            ));
        } else {
            valid_paths.push(path.clone());
        }
    }

    Ok(valid_paths)
}