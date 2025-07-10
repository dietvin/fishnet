/*!
 * This module contains helper functions used during bam and pod5 file loading.
 */

use noodles::{bam::record::Data, sam::alignment::record::data::field::{value::Array, Type, Value}};

use crate::error::loader_errors::bam_errors::BamReadError;

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
/// * `default_value` - The default value to use if an TagNotPresent error occurs
///
/// # Returns
///
/// * `Ok(Some(value))` - If the tag was retrieved successfully or an TagNotPresent error occurred
/// * `Err(e)` - If another error occurred
pub fn unpack_tag<V>(get_tag_result: Result<V, BamReadError>, default_value: Option<V>) -> Result<Option<V>, BamReadError> {
    match get_tag_result {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            match e {
                BamReadError::TagNotPresent(_) => Ok(default_value),
                _ => Err(e)
            }
        }
    }
}


/// Transforms a &str to the format expected from noodles
fn from_str(s: &str) -> Result<[u8; 2], BamReadError> {
    let b = s.as_bytes();
    if b.len() == 2 {
        Ok([b[0], b[1]])
    } else {
        Err(BamReadError::InvalidTagLength(s.to_string()))
    }
}


/// Retrieves an integer array tag from a BAM record.
///
/// This function handles different integer array types (i8, i16, i32) and
/// converts them to a Vec<i16>.
///
/// # Arguments
///
/// * `bam_data` - Data from a bam record
/// * `tag` - The tag name (e.g., "RG", "NM")
///
/// # Returns
///
/// * `Ok(Vec<i16>)` - The integer array if successful
/// * `Err(BamReadError)` - If the tag is missing, has an unexpected type or 
///                         the tag was found, but could not be extracted
pub fn get_iarray_tag(bam_data: &Data, tag: &str) -> Result<Vec<i16>, BamReadError> {
    let tag_b: [u8; 2] = from_str(tag)?;
    match bam_data.get(&tag_b) {
        Some(Ok(Value::Array(Array::Int8(arr)))) => {
            arr.iter()
                .map(|res| res.map(Into::into))
                .collect::<Result<Vec<i16>, _>>()
                .map_err(BamReadError::IoError)
        },
        Some(Ok(Value::Array(Array::Int16(arr)))) => {
            arr.iter()
                .map(|res| res.map(Into::into))
                .collect::<Result<Vec<i16>, _>>()
                .map_err(BamReadError::IoError)
        },
        Some(Ok(Value::Array(Array::Int32(arr)))) => {
            arr.iter()
                .map(|res| {
                    res.and_then(|v| {
                        v.try_into().map_err(|_| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("i32 value {} cannot fit in i16", v)
                            )
                        })
                    }) 
                })
                .collect::<Result<Vec<i16>, _>>()
                .map_err(BamReadError::IoError)
        },
        // The tag was found, but an error occured while extracting it
        Some(Err(e)) => Err(BamReadError::TagNotExtracted(tag.to_string(), e)),
        // The tag was found, but the type does not match
        Some(Ok(other)) => handle_unexpected_type(tag, other.ty(), "UInt"),
        // The tag was not found
        None => Err(BamReadError::TagNotPresent(tag.to_string()))
    }
}

/// Retrieves an unsigned integer tag from a BAM record.
///
/// This function handles different unsigned integer types (u8, u16, u32) and
/// converts them to a usize.
///
/// # Arguments
///
/// * `bam_record` - Data from a bam record
/// * `tag` - The tag name (e.g., "RG", "NM")
///
/// # Returns
///
/// * `Ok(usize)` - The unsigned integer if successful
/// * `Err(BamReadError)` - If the tag is missing, has an unexpected type or 
///                         the tag was found, but could not be extracted
pub fn get_uint_tag(bam_data: &Data, tag: &str) -> Result<usize, BamReadError> {
    let tag_b: [u8; 2] = from_str(tag)?;
    match bam_data.get(&tag_b) {
        Some(Ok(Value::UInt8(value))) => Ok(value as usize),
        Some(Ok(Value::UInt16(value))) => Ok(value as usize),
        Some(Ok(Value::UInt32(value))) => Ok(value as usize),
        Some(Ok(Value::Int8(value))) => Ok(value as usize),
        Some(Ok(Value::Int16(value))) => Ok(value as usize),
        Some(Ok(Value::Int32(value))) => Ok(value as usize),
        // The tag was found, but an error occured while extracting it
        Some(Err(e)) => Err(BamReadError::TagNotExtracted(tag.to_string(), e)),
        // The tag was found, but the type does not match
        Some(Ok(other)) => handle_unexpected_type(tag, other.ty(), "UInt"),
        // The tag was not found
        None => Err(BamReadError::TagNotPresent(tag.to_string()))
    }
}

/// Retrieves a string tag from a BAM record.
///
/// # Arguments
///
/// * `bam_record` - Data from a bam record
/// * `tag` - The tag name (e.g., "RG", "PG")
///
/// # Returns
///
/// * `Ok(String)` - The string value if successful
/// * `Err(BamReadError)` - If the tag is missing, has an unexpected type or 
///                         the tag was found, but could not be extracted
pub fn get_str_tag(bam_data: &Data, tag: &str) -> Result<String, BamReadError> {
    let tag_b: [u8; 2] = from_str(tag)?;
    match bam_data.get(&tag_b) {
        // The tag was found and a float
        Some(Ok(Value::String(value))) => Ok(value.to_string()),
        // The tag was found, but an error occured while extracting it
        Some(Err(e)) => Err(BamReadError::TagNotExtracted(tag.to_string(), e)),
        // The tag was found, but the type does not match
        Some(Ok(other)) => handle_unexpected_type(tag, other.ty(), "String"),
        // The tag was not found
        None => Err(BamReadError::TagNotPresent(tag.to_string()))
    }
}

/// Retrieves a float tag from a BAM record.
///
/// # Arguments
///
/// * `bam_data` - Data from a bam record
/// * `tag` - The tag name (e.g., "AS", "XQ")
///
/// # Returns
///
/// * `Ok(f32)` - The float value if successful
/// * `Err(BamReadError)` - If the tag is missing, has an unexpected type or 
///                         the tag was found, but could not be extracted
pub fn get_float_tag(bam_data: &Data, tag: &str) -> Result<f32, BamReadError> {
    let tag_b: [u8; 2] = from_str(tag)?;
    match bam_data.get(&tag_b) {
        // The tag was found and a float
        Some(Ok(Value::Float(value))) => Ok(value),
        // The tag was found, but an error occured while extracting it
        Some(Err(e)) => Err(BamReadError::TagNotExtracted(tag.to_string(), e)),
        // The tag was found, but the type does not match
        Some(Ok(other)) => handle_unexpected_type(tag, other.ty(), "float"),
        // The tag was not found
        None => Err(BamReadError::TagNotPresent(tag.to_string()))
    }
}

/// Helper function to handle unexpected tag types.
///
/// # Arguments
///
/// * `tag` - The tag name that was being accessed
/// * `value` - The actual value retrieved from the tag
/// * `exp_type` - A string describing the expected type
///
/// # Returns
///
/// * `Err(BamReadError::TagUnexpectedTypeError)` - Always returns an error
fn handle_unexpected_type<V>(tag: &str, obs_type: Type, exp_type: &str) -> Result<V, BamReadError> {
    let type_name = match obs_type {
        Type::Character => "Character",
        Type::Int8 => "Int8",
        Type::UInt8 => "UInt8",
        Type::Int16 => "Int16",
        Type::UInt16 => "UInt16",
        Type::Int32 => "Int32",
        Type::UInt32 => "UInt32",
        Type::Float => "Float",
        Type::String => "String",
        Type::Hex => "Hex",
        Type::Array => "Array",
    }.to_string();
    Err(BamReadError::TagUnexpectedTypeError(
        tag.to_string(), 
        exp_type.to_string(), 
        type_name
    ))
}

/// Sets up the reverse complement of a sequence.
/// 
/// # Arguments
/// * `seq` - sequence that is mapped to the reverse strand
/// 
/// # Returns
/// The reverse complement if the input sequence contained only
/// A, C, G, T and N. An error otherwise.
pub fn reverse_complement(seq: &Vec<u8>) -> Result<Vec<u8>, BamReadError> {
    let mut rev_comp = Vec::with_capacity(seq.len());
    
    for base in seq.iter().rev() {
        let complemented = match base {
            65 => 84, // A to T
            67 => 71, // C to G
            71 => 67, // G to C
            84 => 65, // T to A
            78 => 78,  // N to N (ambiguous base stays ambiguous)
            97 => 84, // a to T
            99 => 71, // c to G
            103 => 67, // g to C
            116 => 65, // t to A
            110 => 78, // n to N (lowercase ambiguous base)

            _ => return Err(BamReadError::ReverseComplement(*base))
        };
        
        rev_comp.push(complemented);
    }
    
    Ok(rev_comp)
}








// ########################################################################################################################
//                                             Helper functions for Pod5Read
// ########################################################################################################################

/// Reverses a vector of signal measurements.
/// 
/// This function takes a vector of signal measurements and returns a new vector
/// with the elements in reverse order.
/// 
/// # Arguments
/// 
/// * `signal` - Vector of current measurements to be reversed
/// 
/// # Returns
/// 
/// A new vector containing the same elements as `signal` but in reverse order
pub fn reverse_signal(signal: &Vec<i16>) -> Vec<i16> {
    signal.iter().rev().map(|el| *el).collect::<Vec<i16>>()
}










// ########################################################################################################################
//                                             Helper functions for Pod5Index
// ########################################################################################################################
use std::{path::{PathBuf, Path}, ffi::OsStr, fs};
use walkdir::WalkDir;
use crate::error::loader_errors::file_handling_errors::{DirHandlingError, FileHandlingError};

/// Finds files with a specific extension in a directory.
/// 
/// This function searches for files with the specified extension in the given directory.
/// It can optionally search recursively through subdirectories.
/// 
/// # Arguments
/// 
/// * `path` - Path to the directory to search in
/// * `file_type` - File extension to search for (without the dot)
/// * `recursive` - Whether to search recursively through subdirectories
/// 
/// # Returns
/// 
/// * `Ok(Vec<String>)` - A vector of file paths if successful
/// * `Err(DirHandlingError)` - If the directory doesn't exist or no matching files are found
pub fn find_files_in_dir(path: &PathBuf, file_type: &str, recursive: bool) -> Result<Vec<PathBuf>,DirHandlingError> {
    let path_obj = Path::new(path);

    // Checks if the path is a directory and if it exists
    if !path_obj.exists() || !path_obj.is_dir() {
        return Err(
            DirHandlingError::DirectoryNotFound(path.clone())
        );
    }

    let mut file_paths = Vec::new();

    // Walk the directory and all sub-directories and extract valid file names
    if recursive {
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok()) {
            let file_path = entry.into_path();

            if valid_type_path(&file_path, file_type) {
                // Maybe include check for non-unicode chars (return error if found)
                file_paths.push(file_path.clone());
            }
        }
    } else {
        // Only search in the given directory
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();
            if valid_type_path(&file_path, file_type) {
                file_paths.push(file_path.clone());
            }
        }
    }

    // Check if valid files were found
    if file_paths.len() > 0 {
        Ok(file_paths)
    } else {
        Err(DirHandlingError::NoFilesFound(file_type.to_string(), path.clone()))
    }
}

/// Checks if a given file path exists and if the file extension is as expected
/// 
/// # Arguments
/// 
/// * `file_path` - Path of the file
/// * `extension` - Expected file extension
/// 
/// # Returns 
/// 
/// True if the file exists and the extension is as expected, false otherwise  
fn valid_type_path(file_path: &PathBuf, extension: &str) -> bool {
    file_path.is_file() && file_path.extension() == Some(OsStr::new(extension))
}

/// Validates and filters a list of file paths based on existence and file type.
/// 
/// This function checks each file path in the provided vector to ensure it exists
/// and has the expected file extension.
/// 
/// # Arguments
/// 
/// * `file_paths` - Vector of file paths to validate
/// * `file_type` - Expected file extension (without the dot)
/// 
/// # Returns
/// 
/// * `Ok(Vec<String>)` - A vector of valid file paths
/// * `Err(FileHandlingError)` - If any file doesn't exist or has an incorrect extension
pub fn get_files(file_paths: &Vec<PathBuf>, file_type: &str) -> Result<Vec<PathBuf>,FileHandlingError> {
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