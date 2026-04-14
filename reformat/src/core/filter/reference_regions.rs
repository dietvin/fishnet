use std::{collections::HashMap, fs::File, io::{BufRead, BufReader}, path::PathBuf};

use crate::{core::filter::{reference_region::ReferenceRegion, MatchedFilterInfo}, error::core::filter::ReferenceRegionsError, execute::config::FilterSource};

/// A collection of genomic regions grouped by reference sequence name.
///
/// This struct is constructed from a [`FilterSource`] such as a BED file,
/// a list of SAM-style region strings, or positions of interest. Regions are stored
/// in a `HashMap` keyed by their reference sequence name (`ref_name`), allowing
/// efficient grouping and lookups by chromosome/scaffold.
#[derive(Debug)]
pub(crate) struct ReferenceRegions {
    /// Groups regions by their sequence name 
    regions: HashMap<String, Vec<ReferenceRegion>>
}

impl ReferenceRegions {
    /// Constructs a new `ReferenceRegions` instance from a given [`FilterSource`].
    ///
    /// Automatically determines the appropriate parsing method based on the
    /// FilterSource variant and constructs the region collection.
    ///
    /// # Arguments
    /// * `filter_source` - The source configuration specifying how to load regions
    ///
    /// # Returns
    /// * `Result<Self, ReferenceRegionsError>` - The constructed regions collection or an error
    ///
    /// # Supported Sources
    /// - [`FilterSource::RefRegionFromBed`] -> Reads regions from a BED file
    /// - [`FilterSource::RefRegionFromInput`] -> Parses SAM-style region strings
    /// - [`FilterSource::PositionsOfInterest`] -> Creates windowed regions around positions
    ///
    /// # Errors
    /// Returns an error if the filter source is invalid or parsing fails.
    pub(crate) fn from_filter_source(filter_source: &FilterSource) -> Result<Self, ReferenceRegionsError> {
        match filter_source {
            FilterSource::RefRegionFromBed { path } => Self::from_bed(path),
            FilterSource::RefRegionFromInput { regions } => Self::from_samstyle_regions(regions),
            FilterSource::PositionsOfInterest { pois } => Self::from_positions_of_interest(pois),
            _ => return Err(ReferenceRegionsError::InvalidFilterSource)
        }
    }

    /// Reads regions from a BED file at the specified path.
    ///
    /// Parses a standard BED format file where each non-comment, non-empty line
    /// contains at least three tab/space-separated fields: `<chrom> <start> <end>`.
    /// Additional fields beyond the first three are ignored. Comments start with '#'.
    ///
    /// # Arguments
    /// * `path` - Path to the BED format file
    ///
    /// # Returns
    /// * `Result<Self, ReferenceRegionsError>` - The constructed regions collection or an error
    ///
    /// # Errors
    /// Returns an error if the file cannot be read, a line cannot be parsed,
    /// or the coordinates are invalid.
    fn from_bed(path: &PathBuf) -> Result<Self, ReferenceRegionsError> {
        let mut regions: HashMap<String, Vec<ReferenceRegion>> = HashMap::new();

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        for line_res in reader.lines() {
            let line = line_res?;

            if line.starts_with("#") || line.trim().is_empty() {
                continue;
            }

            let fields = line.split_whitespace().collect::<Vec<_>>();

            if fields.len() < 3 {
                return Err(ReferenceRegionsError::InvalidBedLine(line));
            }

            let name = fields[0].to_string();
            let start = fields[1].parse::<usize>()?;
            let end = fields[2].parse::<usize>()?;

            let region = ReferenceRegion::from_bed_entry(name.clone(), start, end)?;
            
            regions
                .entry(name)
                .or_default()
                .push(region);
        }

        Ok(Self { regions })
    }

    /// Parses a list of SAM-style region strings.
    ///
    /// Each string should be in the format `"<seq_name>:<start>-<end>"` with
    /// 1-based inclusive coordinates. Regions are grouped by their reference
    /// sequence name for efficient lookups.
    ///
    /// # Arguments
    /// * `region_strings` - Vector of SAM-style region strings to parse
    ///
    /// # Returns
    /// * `Result<Self, ReferenceRegionsError>` - The constructed regions collection or an error
    ///
    /// # Errors
    /// Returns an error if any region string is malformed or contains invalid coordinates.
    fn from_samstyle_regions(region_strings: &Vec<String>) -> Result<Self, ReferenceRegionsError> {
        let mut regions: HashMap<String, Vec<ReferenceRegion>> = HashMap::new();

        for region_string in region_strings {
            let region = ReferenceRegion::from_region_string(region_string.clone())?;
            regions
                .entry(region.name().to_string())
                .or_default()
                .push(region);
        }

        Ok(Self { regions })
    }

    /// Creates windowed regions around positions of interest.
    ///
    /// Each position string should be in the format `"<seq_name>:<position>-<window_size>"`
    /// where position is 1-based and window_size defines the number of bases to
    /// include upstream and downstream of the center position.
    ///
    /// # Arguments
    /// * `poi_strings` - Vector of position strings with window specifications
    ///
    /// # Returns
    /// * `Result<Self, ReferenceRegionsError>` - The constructed regions collection or an error
    ///
    /// # Errors
    /// Returns an error if parsing fails or coordinates are invalid.
    fn from_positions_of_interest(poi_strings: &Vec<String>) -> Result<Self, ReferenceRegionsError> {
        let mut regions: HashMap<String, Vec<ReferenceRegion>> = HashMap::new();

        for region_string in poi_strings {
            let region = ReferenceRegion::from_position_with_window(region_string.clone())?;
            regions
                .entry(region.name().to_string())
                .or_default()
                .push(region);
        }

        Ok(Self { regions })
    }

    /// Checks if any stored regions are fully contained within the given region.
    ///
    /// Searches for regions on the same reference sequence that are completely
    /// contained within the bounds of the provided region. Returns information
    /// about all matching regions including their relative positions.
    ///
    /// # Arguments
    /// * `other` - The potentially containing region to check against
    ///
    /// # Returns
    /// * `Option<Vec<MatchedFilterInfo>>` - Vector of information about contained regions
    ///   if any matches are found, None if no regions are contained
    ///
    /// # Notes
    /// The returned MatchedFilterInfo objects contain start and end positions relative
    /// to the start of the `other` region (i.e., offset coordinates).
    pub(crate) fn self_in_other(&self, other: &ReferenceRegion) -> Option<Vec<MatchedFilterInfo>> {
        let mut hits: Vec<MatchedFilterInfo> = Vec::new();

        if let Some(regions) = self.regions.get(other.name()) {
            for region in regions {
                if region.self_fully_in_other(other) {
                    // TODO: Check if indexing is correct here... (Test failed)
                    let chunk_info = MatchedFilterInfo::new(
                        region.to_samtools_string(), 
                        region.start() - other.start(), 
                        region.end() - other.start() 
                    );
                    hits.push(chunk_info);
                }
            }
        }

        if hits.is_empty() {
            None
        } else {
            Some(hits)
        }
    }

    /// Checks if all reference regions have the same length
    /// 
    /// # Returns 
    /// The length of the regions if all regions have the same length.
    /// None otherwise
    pub(crate) fn equal_len(&self) -> Option<usize> {
        let first_length = match self.regions.iter().next() {
            Some((_,v)) => v[0].length(),
            None => return None
        };

        if self.regions.iter().all(|(_, regions)| {
            regions
                .iter()
                .all(|reg| reg.length() == first_length)
        }) {
            Some(first_length)
        } else {
            None
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_region(chr: &str, start: usize, end: usize) -> ReferenceRegion {
        ReferenceRegion::from_bed_entry(chr.to_string(), start, end).unwrap()
    }

    #[test]
    fn test_from_bed_valid_file() {
        // Create a temporary BED file
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "chr1\t100\t200").unwrap();
        writeln!(tmp, "chr1\t300\t400").unwrap();
        writeln!(tmp, "chr2\t50\t60").unwrap();

        let regions = ReferenceRegions::from_bed(&tmp.path().to_path_buf()).unwrap();

        assert_eq!(regions.regions["chr1"].len(), 2);
        assert_eq!(regions.regions["chr2"].len(), 1);
    }

    #[test]
    fn test_from_bed_invalid_line() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "chr1\t100").unwrap(); // missing end coordinate

        let err = ReferenceRegions::from_bed(&tmp.path().to_path_buf()).unwrap_err();
        match err {
            ReferenceRegionsError::InvalidBedLine(_) => {}
            _ => panic!("Expected InvalidBedLine error"),
        }
    }

    #[test]
    fn test_from_samstyle_regions() {
        let inputs = vec!["chr1:100-200".into(), "chr1:150-180".into(), "chr2:10-20".into()];
        let regions = ReferenceRegions::from_samstyle_regions(&inputs).unwrap();

        assert_eq!(regions.regions["chr1"].len(), 2);
        assert_eq!(regions.regions["chr2"].len(), 1);
    }

    #[test]
    fn test_from_positions_of_interest() {
        let inputs = vec!["chr1:100-4".into(), "chr2:50-4".into()];
        let regions = ReferenceRegions::from_positions_of_interest(&inputs).unwrap();

        assert_eq!(regions.regions["chr1"].len(), 1);
        assert_eq!(regions.regions["chr2"].len(), 1);
    }

    #[test]
    fn test_contains_true() {
        let mut rr = ReferenceRegions { regions: HashMap::new() };
        rr.regions.insert("chr1".into(), vec![make_region("chr1", 120, 150)]);

        let contained = make_region("chr1", 100, 200);
        assert_eq!(rr.self_in_other(&contained), Some(vec![MatchedFilterInfo::new("chr1:121-150".to_string(), 20, 50)]));
    }

    #[test]
    fn test_contains_false_different_chr() {
        let mut rr = ReferenceRegions { regions: HashMap::new() };
        rr.regions.insert("chr1".into(), vec![make_region("chr1", 100, 200)]);

        let other = make_region("chr2", 120, 150);
        assert_eq!(rr.self_in_other(&other), None);
    }

    #[test]
    fn test_contains_false_not_inside() {
        let mut rr = ReferenceRegions { regions: HashMap::new() };
        rr.regions.insert("chr1".into(), vec![make_region("chr1", 100, 200)]);

        let outside = make_region("chr1", 201, 250);
        assert_eq!(rr.self_in_other(&outside), None);
    }
}