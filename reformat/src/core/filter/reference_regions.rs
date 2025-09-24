use std::{collections::HashMap, fs::File, io::{BufRead, BufReader}, path::PathBuf};

use crate::{core::filter::{reference_region::ReferenceRegion, ChunkInfo}, error::core::filter::ReferenceRegionsError, execute::config::FilterSource};

/// A collection of `ReferenceRegion`s, grouped by reference sequence name.
///
/// This struct is constructed from a `FilterSource` such as a BED file,
/// a list of SAM-style region strings, or positions of interest. Regions are stored
/// in a `HashMap` keyed by their reference sequence name (`ref_name`), allowing
/// efficient grouping and lookups.
#[derive(Debug)]
pub(crate) struct ReferenceRegions {
    /// Groups regions by their sequence name 
    regions: HashMap<String, Vec<ReferenceRegion>>
}

impl ReferenceRegions {
    /// Constructs a new [`ReferenceRegions`] instance from a given [`FilterSource`].
    ///
    /// - `FilterSource::RefRegionFromBed` -> Reads regions from a BED file.
    /// - `FilterSource::RefRegionFromInput` -> Parses SAM-style region strings (e.g., `"chr1:100-200"`).
    /// - `FilterSource::PositionsOfInterest` -> Parses from a set window around positions of interest.
    ///
    /// Returns an error if the filter source is invalid or parsing fails.
    pub(crate) fn from_filter_source(filter_source: &FilterSource) -> Result<Self, ReferenceRegionsError> {
        match filter_source {
            FilterSource::RefRegionFromBed { path } => Self::from_bed(path),
            FilterSource::RefRegionFromInput { regions } => Self::from_samstyle_regions(regions),
            FilterSource::PositionsOfInterest { pois } => Self::from_positions_of_interest(pois),
            _ => return Err(ReferenceRegionsError::InvalidFilterSource)
        }
    }

    /// Reads regions from a BED file at the given path.
    ///
    /// Each non-comment, non-empty line must contain at least three fields:
    /// `<chrom> <start> <end>`. Additional fields are ignored.
    ///
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

    /// Parses a list of SAM-style region strings (e.g., `"chr1:100-200"`).
    ///
    /// Each string is parsed into a `ReferenceRegion` using
    /// `ReferenceRegion::from_region_string`. Regions are grouped
    /// by their reference sequence name.
    ///
    /// Returns an error if any region string is malformed.
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

    /// Parses a list of positions of interest and expands them into `ReferenceRegion`s.
    ///
    /// Each string is parsed into a `ReferenceRegion` using
    /// `ReferenceRegion::from_position_with_window`, which expands
    /// the position into a windowed region.
    ///
    /// Returns an error if parsing fails.
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

    /// Checks if one of the regions at hand is fully contained in the given reference
    /// region. If so, returns a String representation of the matching region. Otherwise
    /// returns None.
    pub(crate) fn self_in_other(&self, other: &ReferenceRegion) -> Option<Vec<ChunkInfo>> {
        let mut hits: Vec<ChunkInfo> = Vec::new();

        if let Some(regions) = self.regions.get(other.name()) {
            for region in regions {
                if region.self_fully_in_other(other) {
                    let chunk_info = ChunkInfo::new(
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
        assert_eq!(rr.self_in_other(&contained), Some(vec![ChunkInfo::new("chr1:120-150".to_string(), 20, 50)]));
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