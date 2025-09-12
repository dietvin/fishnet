use crate::core::footer::footer_schema::minknow::reads_format::{
    ContentType, 
    EmbeddedFile, 
    Format
};

/// Simplified enum representing the type of content stored in an embedded file section.
/// Maps the flatbuffers ContentType enum to a more manageable set of variants,
/// treating unrecognized types as `Unknown`.
#[derive(Debug, PartialEq)]
pub enum EmbeddedContentType {
    ReadsTable,
    SignalTable,
    ReadIdIndex,
    OtherIndex,
    Unknown
}

/// Container for metadata about embedded content within a pod5 file.
/// Stores the location, size, format, and content type information
/// extracted from flatbuffers EmbeddedFile objects.
#[derive(Debug)]
pub struct EmbeddedContent {
    offset: i64,
    length: i64,
    format: Format,
    content_type: EmbeddedContentType
}

impl EmbeddedContent {
    /// Creates a new `EmbeddedContent` instance from a flatbuffers `EmbeddedFile` object.
    /// 
    /// This function parses the content type from the flatbuffers schema and maps it to the
    /// simplified `EmbeddedContentType` enum. Any content types not explicitly handled
    /// (ReadsTable, SignalTable, ReadIdIndex, OtherIndex) are mapped to `Unknown`.
    /// 
    /// # Arguments
    /// 
    /// * `embedded_file` - A reference to the flatbuffers `EmbeddedFile` object containing
    ///   the embedded content metadata
    /// 
    /// # Returns
    /// 
    /// A new `EmbeddedContent` instance with the parsed metadata
    pub(crate) fn from_embedded_file(embedded_file: &EmbeddedFile) -> Self {
        let content_type = match embedded_file.content_type() {
            ContentType::ReadsTable => EmbeddedContentType::ReadsTable,
            ContentType::SignalTable => EmbeddedContentType::SignalTable,
            ContentType::ReadIdIndex => EmbeddedContentType::ReadIdIndex,
            ContentType::OtherIndex => EmbeddedContentType::OtherIndex,    
            _ => EmbeddedContentType::Unknown
        };
        EmbeddedContent { 
            offset: embedded_file.offset(), 
            length: embedded_file.length(), 
            format: embedded_file.format(), 
            content_type
        }
    }

    /// Returns the byte offset of the embedded content within the pod5 file.
    /// 
    /// # Returns
    /// 
    /// The offset as a signed 64-bit integer
    pub fn offset(&self) -> i64 {
        self.offset
    }

    /// Returns the length in bytes of the embedded content.
    /// 
    /// # Returns
    /// 
    /// The length as a signed 64-bit integer
    pub fn length(&self) -> i64 {
        self.length
    }

    /// Returns a reference to the format specification of the embedded content.
    /// 
    /// This indicates how the embedded data is structured and encoded.
    /// 
    /// # Returns
    /// 
    /// A reference to the `Format` enum value
    pub fn format(&self) -> &Format {
        &self.format
    }

    /// Returns a reference to the content type of the embedded data.
    /// 
    /// The content type indicates what kind of data is stored in this embedded section
    /// (e.g., reads table, signal table, indices, etc.).
    /// 
    /// # Returns
    /// 
    /// A reference to the `EmbeddedContentType` enum value
    pub fn content_type(&self) -> &EmbeddedContentType {
        &self.content_type
    }
}