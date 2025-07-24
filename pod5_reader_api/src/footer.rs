mod footer_schema;
pub mod embedded_content;

use std::{fs::File, io::{Read, Seek, SeekFrom}};
use flatbuffers::{root, InvalidFlatbuffer};
use crate::{
    footer::{
        embedded_content::{
            EmbeddedContent, 
            EmbeddedContentType
        }, 
        footer_schema::minknow::reads_format::Footer
    }
};

/// Parser for pod5 file footer data that provides access to file metadata and embedded content information.
/// Contains file identifier, software version, pod5 format version, and a list of embedded FeatherV2 datasets
/// (typically reads table, signal table, and run info table).
#[derive(Debug)]
pub struct Pod5Footer {
    file_identifier: String,
    software: String,
    pod5_version: String,
    contents: Vec<EmbeddedContent>
}

impl Pod5Footer {
    /// Creates a new `Pod5Footer` instance by parsing the footer data from a pod5 file.
    /// 
    /// This function follows the pod5 file specification to locate and parse the footer:
    /// 1. Reads the footer length from 32 bytes before the end of the file
    /// 2. Seeks to the footer location and reads the footer data
    /// 3. Parses the flatbuffers Footer structure
    /// 4. Extracts metadata (file identifier, software, version) and embedded content information
    /// 
    /// # Arguments
    /// 
    /// * `file` - A mutable reference to the pod5 file to read from
    /// 
    /// # Returns
    /// 
    /// A `Result` containing the new `Pod5Footer` instance on success, or a
    /// `Pod5FooterError` if parsing fails
    /// 
    /// # Errors
    /// 
    /// * `Pod5FooterError::IoError` - If file I/O operations fail
    /// * `Pod5FooterError::FlatBuffersError` - If the footer data is corrupted or invalid
    /// * `Pod5FooterError::EmptyFileIdentifier` - If the file identifier is missing
    /// * `Pod5FooterError::EmptySoftware` - If the software field is missing
    /// * `Pod5FooterError::EmptyVersion` - If the pod5 version is missing
    /// * `Pod5FooterError::EmptyContents` - If the contents section is missing
    /// * `Pod5FooterError::EmptyContentsVec` - If the contents vector is empty
    pub fn new(file: &mut File) -> Result<Self, Pod5FooterError> {
        let footer_len = Pod5Footer::get_footer_len(file)?;
        let mut footer_buf = vec![0u8; footer_len as usize];

        file.seek(SeekFrom::End(-32-footer_len))?;
        file.read(&mut footer_buf)?;

        let footer = root::<Footer>(&footer_buf)?;

        let file_identifier = footer
            .file_identifier()
            .ok_or(Pod5FooterError::EmptyFileIdentifier)?
            .to_string();

        let software = footer
            .software()
            .ok_or(Pod5FooterError::EmptySoftware)?
            .to_string();

        let pod5_version = footer
            .pod5_version()
            .ok_or(Pod5FooterError::EmptyVersion)?
            .to_string();

        let contents = Pod5Footer::parse_contents(&footer)?;

        Ok(Pod5Footer { 
            file_identifier,
            software,
            pod5_version,
            contents
        })
    }

    /// Extracts the footer length from the pod5 file.
    /// 
    /// Reads an 8-byte little-endian integer from 32 bytes before the end of the file,
    /// which contains the size of the footer data according to the pod5 specification.
    /// 
    /// # Arguments
    /// 
    /// * `file` - A mutable reference to the pod5 file to read from
    /// 
    /// # Returns
    /// 
    /// A `Result` containing the footer length in bytes, or a `Pod5FooterError` if reading fails
    /// 
    /// # Errors
    /// 
    /// * `Pod5FooterError::IoError` - If file I/O operations fail
    fn get_footer_len(file: &mut File) -> Result<i64, Pod5FooterError> {
        let mut footer_len_buf: [u8; 8] = [0; 8];
        file.seek(std::io::SeekFrom::End(-32))?; 
        file.read(&mut footer_len_buf)?;

        Ok(i64::from_le_bytes(footer_len_buf))
    }

    /// Parses embedded file information from the flatbuffers Footer into a vector of EmbeddedContent objects.
    /// 
    /// Converts the flatbuffers representation of embedded files into a more accessible format.
    /// The pod5 format typically contains three FeatherV2 datasets: reads table, signal table,
    /// and run info table.
    /// 
    /// # Arguments
    /// 
    /// * `footer` - A reference to the parsed flatbuffers Footer object
    /// 
    /// # Returns
    /// 
    /// A `Result` containing a vector of `EmbeddedContent` objects, or a `Pod5FooterError`
    /// if the contents are missing or empty
    /// 
    /// # Errors
    /// 
    /// * `Pod5FooterError::EmptyContents` - If the contents field is missing from the footer
    /// * `Pod5FooterError::EmptyContentsVec` - If the contents vector is empty
    fn parse_contents(footer: &Footer) -> Result<Vec<EmbeddedContent>, Pod5FooterError> {
        let contents = footer.contents().ok_or(
            Pod5FooterError::EmptyContents
        )?;

        if contents.len() == 0 {
            return Err(Pod5FooterError::EmptyContentsVec);
        }

        let mut embedded_content = Vec::with_capacity(contents.len());
        for emb_file in contents {
            let embedded_file = EmbeddedContent::from_embedded_file(&emb_file);
            embedded_content.push(embedded_file);
        }

        Ok(embedded_content)
    }

    /// Returns the file identifier string from the pod5 footer.
    /// 
    /// The file identifier indicates the format and version of the pod5 file.
    /// 
    /// # Returns
    /// 
    /// A string slice containing the file identifier
    pub fn file_identifier(&self) -> &str {
        self.file_identifier.as_str()
    }

    /// Returns the software name that created the pod5 file.
    /// 
    /// This field identifies the software package and version used to generate the file.
    /// 
    /// # Returns
    /// 
    /// A string slice containing the software information
    pub fn software(&self) -> &str {
        self.software.as_str()
    }

    /// Returns the pod5 format version used in the file.
    /// 
    /// This indicates which version of the pod5 specification the file conforms to.
    /// 
    /// # Returns
    /// 
    /// A string slice containing the pod5 version
    pub fn pod5_version(&self) -> &str {
        self.pod5_version.as_str()
    }

    /// Returns a reference to the vector of embedded content information.
    /// 
    /// This provides access to metadata about all embedded FeatherV2 datasets
    /// contained within the pod5 file.
    /// 
    /// # Returns
    /// 
    /// A reference to the vector of `EmbeddedContent` objects
    pub fn contents(&self) -> &Vec<EmbeddedContent> {
        &self.contents
    }

    /// Retrieves an embedded content object of the specified type.
    /// 
    /// Searches through the embedded content list to find the first entry matching
    /// the requested content type (e.g., ReadsTable, SignalTable, ReadIdIndex).
    /// 
    /// # Arguments
    /// 
    /// * `file_type` - The type of embedded content to retrieve
    /// 
    /// # Returns
    /// 
    /// A `Result` containing a reference to the matching `EmbeddedContent`, or a
    /// `Pod5FooterError` if no content of the specified type is found
    /// 
    /// # Errors
    /// 
    /// * `Pod5FooterError::EmbeddedFileNotFound` - If no embedded file of the specified type exists
    pub fn retrieve_embedded_file(&self, file_type: EmbeddedContentType) -> Result<&EmbeddedContent, Pod5FooterError> {
        for embedded_file in &self.contents {
            if file_type == *embedded_file.content_type() {
                return Ok(embedded_file);
            }
        }
        Err(Pod5FooterError::EmbeddedFileNotFound(file_type))
    }
}


/// Error types that can occur during pod5 footer parsing operations.
/// Covers I/O failures, flatbuffers parsing errors, missing required fields,
/// and embedded file lookup failures.
#[derive(Debug, thiserror::Error)]
pub enum Pod5FooterError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid Flatbuffer: {0}")]
    FlatBuffersError(#[from] InvalidFlatbuffer),
    #[error("Empty file identifier")]
    EmptyFileIdentifier,
    #[error("Empty software entry")]
    EmptySoftware,
    #[error("Empty pod5 version entry")]
    EmptyVersion,
    #[error("Empty contents entry")]
    EmptyContents,
    #[error("Contents vector is empty")]
    EmptyContentsVec,
    #[error("Could not file embedded file of type: {0:?}")]
    EmbeddedFileNotFound(EmbeddedContentType)
}