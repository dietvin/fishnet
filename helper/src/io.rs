use crate::errors::PathError;

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Parquet,
    Json,
    Tsv
}

impl OutputFormat {
    pub(crate) fn from_ext(ext: &str, valid_ext: Vec<OutputFormat>) -> Result<Self, PathError> {
        let format = match ext {
            "json" => Self::Json,
            "parquet" => Self::Parquet,
            "tsv" => Self::Tsv,
            _ => return Err(PathError::InvalidExtension(ext.to_string()))
        };

        if valid_ext.contains(&format) {
            Ok(format)
        } else {
            Err(PathError::UnexpectedFormat(ext.to_string(), valid_ext))
        }
    }
}