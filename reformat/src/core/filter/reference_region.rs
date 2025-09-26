use crate::error::core::filter::ReferenceRegionError;

/// Internal representation of a genomic region.
///
/// Coordinates are always stored in **BED-style**:
/// - 0-based indexing
/// - start is inclusive
/// - end is exclusive
///
/// This means that the length of a region is always `end - start`.
/// 
/// Example:
/// ```text
/// Reference sequence: A C G T A T A C C T
/// 0-based index:      0 1 2 3 4 5 6 7 8 9  
///
/// Region: 2-7             G T A T A
/// ```
#[derive(Debug, Clone)]
pub(crate) struct ReferenceRegion {
    name: String,
    start: usize,
    end: usize
}

impl ReferenceRegion {
    /// Constructs a region from BED-style coordinates.
    ///
    /// Creates a region using 0-based indexing with a half-open interval
    /// [start, end) where start is inclusive and end is exclusive.
    ///
    /// # Arguments
    /// * `name` - Reference sequence name (e.g., "chr1", "scaffold_1")
    /// * `start` - 0-based start position (inclusive)
    /// * `end` - 0-based end position (exclusive)
    ///
    /// # Returns
    /// * `Result<Self, ReferenceRegionError>` - The constructed region or an error
    ///
    /// # Errors
    /// Returns `ReferenceRegionError::InvalidCoordinatesBedStyle` if
    /// `start >= end`, since a BED-style region must contain at least one base.
    pub(crate) fn from_bed_entry(name: String, start: usize, end: usize) -> Result<Self, ReferenceRegionError> {
        if start >= end {
            return Err(ReferenceRegionError::InvalidCoordinatesBedStyle(start, end));
        }

        Ok(Self { 
            name,
            start,
            end
        })
    }

    /// Constructs a region from a SAM-style region string.
    ///
    /// Parses strings of the form `"<SEQ-NAME>:<START>-<END>"` where coordinates
    /// are 1-based and inclusive on both ends. These are internally converted
    /// to BED-style (0-based, half-open) coordinates.
    ///
    /// # Arguments
    /// * `region_string` - SAM-style region string (e.g., "chr1:100-200")
    ///
    /// # Returns
    /// * `Result<Self, ReferenceRegionError>` - The constructed region or an error
    ///
    /// # Example
    /// ```text
    /// Input: "chr1:2-7" → bases 2..=7 (1-based inclusive)
    /// Output: start = 1, end = 7 (0-based half-open)
    /// ```
    ///
    /// # Errors
    /// * `ReferenceRegionError::InvalidSamStart` if `start == 0`
    /// * `ReferenceRegionError::InvalidCoordinatesSamStyle` if `start > end`
    /// * `ReferenceRegionError::FromStringInvalidFormat` if parsing fails
    pub(crate) fn from_region_string(region_string: String) -> Result<Self, ReferenceRegionError> {
        let (name, start, end) = Self::parse_string(region_string)?;

        if start == 0 {
            return Err(ReferenceRegionError::InvalidSamStart);
        }
        if start > end {
            return Err(ReferenceRegionError::InvalidCoordinatesSamStyle(start, end));
        }

        Ok(Self { 
            name, 
            start: start - 1, // 1-based inclusive -> 0-based inclusive
            end               // 1-based inclusive -> 0-based exclusive (same value)
        })
    }

    /// Constructs a region from a position with a symmetric window.
    ///
    /// Parses strings of the form `"<SEQ-NAME>:<SITE>-<WINDOW-HALF-SIZE>"` where
    /// SITE is the 1-based center position and WINDOW-HALF-SIZE defines how many
    /// bases upstream and downstream to include.
    ///
    /// # Arguments
    /// * `pos_with_window` - Position string with window size (e.g., "chr1:100-4")
    ///
    /// # Returns
    /// * `Result<Self, ReferenceRegionError>` - The constructed region or an error
    ///
    /// # Example
    /// ```text
    /// Input: "chr1:5-2" → center = 5, window half size = 2
    /// Output: region covering bases 3..=7 (1-based)
    ///         which corresponds to start = 2, end = 7 (0-based)
    /// ```
    ///
    /// # Errors
    /// * `ReferenceRegionError::InvalidSamStart` if `SITE == 0`
    /// * `ReferenceRegionError::FromStringInvalidFormat` if parsing fails
    pub(crate) fn from_position_with_window(pos_with_window: String) -> Result<Self, ReferenceRegionError> {
        let (name, start, size) = Self::parse_string(pos_with_window)?;

        if start == 0 {
            return Err(ReferenceRegionError::InvalidSamStart);
        }

        // 0-based start and end coordinates
        let region_start = start.saturating_sub(size).saturating_sub(1); // inclusive
        let region_end = start + size; // exclusive

        Ok(Self { 
            name, 
            start: region_start, 
            end: region_end 
        })
    }

    /// Constructs a region from a 1-based start position and length.
    ///
    /// The start coordinate is 1-based and inclusive. The length must be greater
    /// than 0. Coordinates are internally converted to BED-style.
    ///
    /// # Arguments
    /// * `name` - Reference sequence name
    /// * `start` - 1-based start position (inclusive)
    /// * `length` - Length of the region in bases
    ///
    /// # Returns
    /// * `Result<Self, ReferenceRegionError>` - The constructed region or an error
    ///
    /// # Example
    /// ```text
    /// Input: name = "chr1", start = 3, length = 4
    /// Output: region covering bases 3..=6 (1-based)
    ///         which corresponds to start = 2, end = 6 (0-based)
    /// ```
    ///
    /// # Errors
    /// * `ReferenceRegionError::InvalidSamStart` if `start == 0`
    /// * `ReferenceRegionError::InvalidLength` if `length == 0`
    pub(crate) fn from_start_and_length(name: String, start: usize, length: usize) -> Result<Self, ReferenceRegionError> {
        if start == 0 {
            return Err(ReferenceRegionError::InvalidSamStart);
        }

        if length == 0 {
            return Err(ReferenceRegionError::InvalidLength);
        }

        let start = start - 1;
        let end = start + length;

        Ok(Self { 
            name, 
            start, 
            end 
        })
    }

    /// Parses a region string into its components.
    ///
    /// Extracts sequence name and coordinate values from strings of the form
    /// `"<SEQ-NAME>:<START>-<END>"`. Does not perform coordinate validation.
    ///
    /// # Arguments
    /// * `region_string` - The region string to parse
    ///
    /// # Returns
    /// * `Result<(String, usize, usize), ReferenceRegionError>` - Tuple of
    ///   (sequence_name, start, end) or an error
    ///
    /// # Errors
    /// Returns `ReferenceRegionError::FromStringInvalidFormat` if the format
    /// is invalid or numeric parsing fails.
    fn parse_string(region_string: String) -> Result<(String, usize, usize), ReferenceRegionError> {
        let (seq_name, range_part) = region_string
            .split_once(":")
            .ok_or(ReferenceRegionError::FromStringInvalidFormat(region_string.clone(), "':' not found"))?;

        let (start_str, end_str) = range_part
            .split_once("-")
            .ok_or(ReferenceRegionError::FromStringInvalidFormat(region_string.clone(), "'-' not found"))?;

        let start = start_str.parse::<usize>()
            .map_err(|_| ReferenceRegionError::FromStringInvalidFormat(
                region_string.clone(), "Failed to parse start coordinate"
            ))?;

        let end = end_str.parse::<usize>()
        .map_err(|_| ReferenceRegionError::FromStringInvalidFormat(
            region_string.clone(), "Failed to parse end coordinate"
        ))?;

        Ok((seq_name.to_string(), start, end))
    }

    /// Checks if this region is fully contained within another region.
    ///
    /// Both regions must be on the same reference sequence. This region is
    /// considered contained if its start >= other.start and its end <= other.end.
    ///
    /// # Arguments
    /// * `other` - The potentially containing region to check against
    ///
    /// # Returns
    /// * `bool` - True if this region is fully contained within `other`
    pub(crate) fn self_fully_in_other(&self, other: &ReferenceRegion) -> bool {
        other.name == self.name && other.start <= self.start && other.end >= self.end
    }

    /// Returns the reference sequence name.
    ///
    /// # Returns
    /// * `&str` - Reference to the sequence name
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Returns the start coordinate (0-based, inclusive).
    ///
    /// # Returns
    /// * `usize` - The start position in BED-style coordinates
    pub(crate) fn start(&self) -> usize {
        self.start
    }

    /// Returns the end coordinate (0-based, exclusive).
    ///
    /// # Returns
    /// * `usize` - The end position in BED-style coordinates
    pub(crate) fn end(&self) -> usize {
        self.end
    }

    /// Returns the length of the region in bases.
    ///
    /// # Returns
    /// * `usize` - The length calculated as `end - start`
    pub(crate) fn length(&self) -> usize {
        self.end - self.start
    }

    /// Formats the region as a samtools-style string.
    ///
    /// Creates a string representation using 0-based coordinates in the format
    /// `"<name>:<start>-<end>"`.
    ///
    /// # Returns
    /// * `String` - The formatted region string
    pub(crate) fn to_samtools_string(&self) -> String {
        format!("{}:{}-{}", self.name, self.start+1, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bed_entry_valid() {
        let region = ReferenceRegion::from_bed_entry("chr1".into(), 2, 7).unwrap();
        assert_eq!(region.name, "chr1");
        assert_eq!(region.start, 2);
        assert_eq!(region.end, 7);
    }

    #[test]
    fn test_from_bed_entry_invalid() {
        let err = ReferenceRegion::from_bed_entry("chr1".into(), 5, 5).unwrap_err();
        matches!(err, ReferenceRegionError::InvalidCoordinatesBedStyle(5, 5));
    }

    #[test]
    fn test_from_region_string_valid() {
        // SAM-style input: chr1:2-7 → bases 2..=7 (1-based) → [1..7) (0-based)
        let region = ReferenceRegion::from_region_string("chr1:2-7".into()).unwrap();
        assert_eq!(region.name, "chr1");
        assert_eq!(region.start, 1);
        assert_eq!(region.end, 7);
    }

    #[test]
    fn test_from_region_string_invalid_start_zero() {
        let err = ReferenceRegion::from_region_string("chr1:0-10".into()).unwrap_err();
        matches!(err, ReferenceRegionError::InvalidSamStart);
    }

    #[test]
    fn test_from_region_string_invalid_range() {
        let err = ReferenceRegion::from_region_string("chr1:10-5".into()).unwrap_err();
        matches!(err, ReferenceRegionError::InvalidCoordinatesSamStyle(10, 5));
    }

    #[test]
    fn test_from_position_with_window_valid() {
        // chr1:5-2 → center = 5, window = 2 → bases 3..=7 → [2..7) in 0-based
        let region = ReferenceRegion::from_position_with_window("chr1:5-2".into()).unwrap();
        assert_eq!(region.name, "chr1");
        assert_eq!(region.start, 2);
        assert_eq!(region.end, 7);
    }

    #[test]
    fn test_from_position_with_window_start_zero() {
        let err = ReferenceRegion::from_position_with_window("chr1:0-2".into()).unwrap_err();
        matches!(err, ReferenceRegionError::InvalidSamStart);
    }

    #[test]
    fn test_from_start_and_length_valid() {
        // start=3 (1-based), length=4 → bases 3..=6 → [2..6) in 0-based
        let region = ReferenceRegion::from_start_and_length("chr1".into(), 3, 4).unwrap();
        assert_eq!(region.name, "chr1");
        assert_eq!(region.start, 2);
        assert_eq!(region.end, 6);
    }

    #[test]
    fn test_from_start_and_length_invalid_start_zero() {
        let err = ReferenceRegion::from_start_and_length("chr1".into(), 0, 5).unwrap_err();
        matches!(err, ReferenceRegionError::InvalidSamStart);
    }

    #[test]
    fn test_from_start_and_length_invalid_length_zero() {
        let err = ReferenceRegion::from_start_and_length("chr1".into(), 5, 0).unwrap_err();
        matches!(err, ReferenceRegionError::InvalidLength);
    }

    #[test]
    fn test_parse_string_valid() {
        let (name, start, end) = ReferenceRegion::parse_string("chr1:10-20".into()).unwrap();
        assert_eq!(name, "chr1");
        assert_eq!(start, 10);
        assert_eq!(end, 20);
    }

    #[test]
    fn test_parse_string_missing_colon() {
        let err = ReferenceRegion::parse_string("chr1-10".into()).unwrap_err();
        matches!(err, ReferenceRegionError::FromStringInvalidFormat(_, _));
    }

    #[test]
    fn test_parse_string_missing_dash() {
        let err = ReferenceRegion::parse_string("chr1:10".into()).unwrap_err();
        matches!(err, ReferenceRegionError::FromStringInvalidFormat(_, _));
    }

    #[test]
    fn test_fully_contains_true() {
        let outer = ReferenceRegion::from_bed_entry("chr1".into(), 4, 7).unwrap();
        let inner = ReferenceRegion::from_bed_entry("chr1".into(), 2, 10).unwrap();
        assert!(outer.self_fully_in_other(&inner));
    }

    #[test]
    fn test_fully_contains_false_different_seq() {
        let outer = ReferenceRegion::from_bed_entry("chr2".into(), 4, 7).unwrap();
        let inner = ReferenceRegion::from_bed_entry("chr1".into(), 2, 10).unwrap();
        assert!(!outer.self_fully_in_other(&inner));
    }

    #[test]
    fn test_fully_contains_false_partial_overlap() {
        let outer = ReferenceRegion::from_bed_entry("chr1".into(), 8, 12).unwrap();
        let inner = ReferenceRegion::from_bed_entry("chr1".into(), 2, 10).unwrap();
        assert!(!outer.self_fully_in_other(&inner));
    }
}