/*!
 * This module contains helper functions for file handling and checking
 */

 use std::{env, fs, path::PathBuf};
 use crate::{errors::{PathError, Pod5PathError}, io::OutputFormat};
 
 /// Checks if an input file exists, if it is a file and if the extension is as expected.
 pub fn check_input_file(input: &PathBuf, expected_ext: &str) -> Result<(), PathError> {
     if !input.exists() {
         return Err(
             PathError::DoesNotExist(input.clone())
         );
     } 
     else if !input.is_file() {
         return Err(
             PathError::IsNotFile(input.clone())
         );
     } 
     else if !valid_extention(input, expected_ext) {
        return Err(
            PathError::ExtensionNone(input.clone())
        );
     }
     
     Ok(())
 }
 
 
 /// Parses the given pod5 input that can consist of both file and directory paths and
 /// converts these to only file paths.
 pub fn check_and_get_pod5_input(inputs: Vec<PathBuf>) -> Result<Vec<PathBuf>, Pod5PathError> {
     log::info!("Parsing {} paths given", inputs.len());
 
     let mut file_paths = vec![];
 
     for path in inputs {
         let abs_path = match path.canonicalize() {
             Ok(canonical) => canonical,
             Err(e) => {
                 log::warn!("Failed to canonicalize path '{}': {}", path.display(), e);
                 continue;
             }
         };
 
         if abs_path.is_dir() {
             let mut dir_files = parse_directory(&abs_path, "pod5");
             file_paths.append(&mut dir_files);
         } if valid_file(&abs_path, "pod5") {
             file_paths.push(abs_path.to_path_buf());
         }
     }
 
     if file_paths.is_empty() {
         return Err(Pod5PathError::NoValidFilesFound);
     }
 
     log::info!("Found {} pod5 file(s)", file_paths.len());
 
     Ok(file_paths)
 }
 
 
 /// Writes all pod5 files in a given directory to a vector
 fn parse_directory(dirname: &PathBuf, target_ext: &str) -> Vec<PathBuf> {
     let mut pod5_files = vec![];
     if let Ok(entries) = fs::read_dir(dirname) {
         for entry in entries.flatten() {
             let path = entry.path();
 
             let abs_path = match path.canonicalize() {
                 Ok(canonical) => canonical,
                 Err(e) => {
                     log::warn!("Failed to canonicalize path '{}': {}", path.display(), e);
                     continue;
                 }
             };
 
             if abs_path.is_dir() {
                 let mut subdir_files = parse_directory(&abs_path, target_ext);
                 pod5_files.append(&mut subdir_files);
             } else if valid_file(&abs_path, target_ext) {
                 pod5_files.push(abs_path);
             }
         }
     } else {
         log::warn!("Failed to read directory: {:?}", dirname);
     }
     pod5_files
 }
 
 
 /// Checks if a given file is a valid pod5 file.
 /// 
 /// Checks if the path is a file, if the file exists and if the extension is 'pod5'.
 /// 
 /// # Arguments
 /// * `path` - Path to a potential pod5 file
 /// 
 /// # Returns
 /// `true` if the path points to a valid pod5 file
 fn valid_file(path: &PathBuf, target_ext: &str) -> bool {
     // Check if the path if a filepath
     if !path.is_file() {
         log::warn!("Invalid file path: '{:?}' (Path is not a file path)", path.to_str());
         false
     } 
     // Check if the file exists
     else if !path.exists() {
         log::warn!("Invalid file path: '{:?}' (File does not exist)", path.to_str());
         false
     } 
     // Check if the file is a pod5 file
     else if !valid_extention(path, target_ext) {
         log::warn!("Invalid file path: '{:?}' (Expected 'pod5' extension)", path.to_str());
         false
     } else {
         true
     }
 }
 
 
 /// Checks if the extension is 'pod5'
 /// 
 /// # Arguments
 /// * `path` - Path to a potential pod5 file
 /// 
 /// # Returns
 /// `true` if the file path has the extension 'pod5'
 fn valid_extention(path: &PathBuf, target_ext: &str) -> bool {
     if let Some(ext) = path.extension() {
         if ext == target_ext {
             return true;
         }
     }
     false
 }
 
 
 /// Checks whether the output file is a file, has the correct 
 pub fn check_output_file(output_file: &PathBuf, force_overwrite: bool, valid_formats: Vec<OutputFormat>) -> Result<(PathBuf, OutputFormat), PathError> {
     // Make path absolute
     let absolute_path  = if output_file.is_absolute() {
         output_file.clone()
     } else {
         let pwd = env::current_dir()?;
         pwd.join(output_file)
     };
 
     // Check if path ends with '/' or points to a directory
     if absolute_path
         .metadata()
         .map(|meta| meta.is_dir())
         .unwrap_or(false) 
     {
         return Err(PathError::IsDir(absolute_path))
     }
 
     // Check base directory exists
     if let Some(parent) = absolute_path.parent() {
         if !parent.exists() {
             return Err(PathError::BaseDirNotExist(absolute_path));
         }
     } else {
         return Err(PathError::BaseDirNotExist(absolute_path));
     }
 
     // Check file extension
     let file_format = match absolute_path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => {
            OutputFormat::from_ext(ext, valid_formats)
        }
        _ => return Err(PathError::ExtensionNone(absolute_path))
     }?;
  
     // Check if file exists and overwrite is not allowed
     if absolute_path.exists() && !force_overwrite {
         return Err(PathError::FileExists(absolute_path));
     }
 
     Ok((absolute_path, file_format))
 }