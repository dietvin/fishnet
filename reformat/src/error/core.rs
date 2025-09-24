pub(crate) mod loader {
    use pod5_reader_api::error::{dataset::Pod5DatasetError, read::Pod5ReadError};

    use crate::{error::core::filter::ReferenceRegionError, execute::config::Column};

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ColumnIndexError {
        #[error("Unexpected field in schema: {0}")]
        UnexpectedFieldName(String),
        #[error("Need column '{1:?}' for {0} data, but column was not found")]
        MissingColumn(&'static str, Column)
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum AlignmentChunkError {
        #[error("No data found for column {0} at index {1}")]
        ColumnIndexError(&'static str, usize),
        #[error("Failed to downcast to {0}")]
        DowncastError(&'static str),
        #[error("Value is None")]
        ValueNone,
        #[error("Uuid error: {0}")]
        UuidError(#[from] uuid::Error),
        #[error("Pod5Dataset is needed, but is None")]
        Pod5DatasetMissing,
        #[error("Invalid index {0} with length {1}")]
        InvalidIndex(usize, usize),
        #[error("Pod5Dataset error: {0}")]
        Pod5DatasetError(#[from] Pod5DatasetError),
        #[error("Pod5Read error: {0}")]
        Pod5ReadError(#[from] Pod5ReadError),
        #[error("Row error: {0}")]
        RowError(#[from] RowError),
        #[error("0-division occured during normalization")]
        ZeroDivision
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum RowError {
        #[error("Reference region error: {0}")]
        ReferenceRegionError(#[from] ReferenceRegionError)
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum RowIteratorError {
        #[error("IO error: {0}")]
        IoError(#[from] std::io::Error),
        #[error("Arrow2 error: {0}")]
        ArrowError(#[from] arrow2::error::Error),
        #[error("Not a single chunk found")]
        NoChunks,
        #[error("Column index error: {0}")]
        ColumnIndexError(#[from] ColumnIndexError),
        #[error("Alignment chunk error: {0}")]
        AlignmentChunkError(#[from] AlignmentChunkError)
    }
}

pub(crate) mod filter {
    use std::num::ParseIntError;

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ReferenceRegionError {
        #[error("Invalid coordinates. Start ({0}) must be smaller than end ({1})")]
        InvalidCoordinatesBedStyle(usize, usize),
        #[error("Invalid coordinates. Start ({0}) must be smaller than or equal to end ({1})")]
        InvalidCoordinatesSamStyle(usize, usize),
        #[error("Failed to parse reference region from string '{0}': {1}")]
        FromStringInvalidFormat(String, &'static str),
        #[error("Sam-style coordinates can not start with 0, since they are 1-based")]
        InvalidSamStart,
        #[error("Length must be larger than 0")]
        InvalidLength
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ReferenceRegionsError {
        #[error("Invalid filter source. Can not parse reference regions from motifs")]
        InvalidFilterSource,
        #[error("IO error: {0}")]
        IoError(#[from] std::io::Error),
        #[error("Invalid bed line: {0}")]
        InvalidBedLine(String),
        #[error("Failed to parse bed entry to usize")]
        ParseUsizeError(#[from] ParseIntError),
        #[error("Reference region error: {0}")]
        RefRegionError(#[from] ReferenceRegionError),
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum MotifError {
        #[error("Motif contains invalid characters. Only 'A', 'C', 'G' and 'U'/'T' is allowed.")]
        InvalidChars
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum MotifsError {
        #[error("Invalid filter source. Can not parse motifs from reference regions")]
        InvalidFilterSource,
        #[error("IO error: {0}")]
        IoError(#[from] std::io::Error),
        #[error("Motif error: {0}")]
        MotifError(#[from] MotifError)
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum FilterError {
        #[error("Motifs error: {0}")]
        MotifsError(#[from] MotifsError),
        #[error("Reference regions error: {0}")]
        RefRegionsError(#[from] ReferenceRegionsError),
        #[error("Filtering for reference regions, but no region was found in the current row")]
        NoRegionInTarget,
        #[error("Filtering for motifs, but only a placeholder sequence was found in the current row")]
        NoSequenceInTarget
    }

}

pub(crate) mod reformat {
    #[derive(Debug, thiserror::Error)]
    pub(crate) enum StatError {
        #[error("Empty vector")]
        VecEmpty
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ReadWiseStatsError {

    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum InterpolationError {

    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ReformatError {
        #[error("Stat error: {0}")]
        StatError(#[from] StatError),
        #[error("Read wise stats error: {0}")]
        ReadWiseStatsError(#[from] ReadWiseStatsError),
        #[error("Interpolation error: {0}")]
        InterpolationError(#[from] InterpolationError),
    }
}