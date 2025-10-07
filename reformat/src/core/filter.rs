use crate::{
    core::{
        alignment_loader::row::Row, 
        filter::{
            motifs::Motifs, 
            reference_regions::ReferenceRegions
        }
    }, 
    error::core::filter::FilterError, 
    execute::config::FilterSource
};

pub(crate) mod reference_region;
pub(crate) mod reference_regions;
pub(crate) mod motif;
pub(crate) mod motifs;

/// Unified interface for different types of sequence filters.
///
/// Supports both reference region-based filtering (for genomic coordinates)
/// and motif-based filtering (for sequence pattern matching).
#[derive(Debug)]
pub(crate) enum Filter {
    ReferenceRegions { regions: ReferenceRegions },
    Motifs { motifs: Motifs}
}

impl Filter {
    /// Constructs a Filter instance from the given configuration.
    ///
    /// Automatically determines the appropriate filter type based on the
    /// FilterSource variant and initializes the corresponding filter.
    ///
    /// # Arguments
    /// * `filter_source` - The source configuration specifying filter type and data
    ///
    /// # Returns
    /// * `Result<Self, FilterError>` - The constructed Filter instance or an error
    ///
    /// # Errors
    /// Returns an error if filter construction fails.
    pub(crate) fn from_filter_source(filter_source: &FilterSource) -> Result<Self, FilterError> {
        match filter_source {
            FilterSource::RefRegionFromBed { .. } | 
            FilterSource::RefRegionFromInput { .. } | 
            FilterSource::PositionsOfInterest { .. } => Ok(Self::ReferenceRegions {
                regions: ReferenceRegions::from_filter_source(filter_source)?
            }),
            FilterSource::MotifFromFile { .. } |
            FilterSource::MotifFromInput { .. } => Ok(Self::Motifs { 
                motifs: Motifs::from_filter_source(filter_source)?
            })
        }
    }

    /// Checks if this filter matches the given row and returns hit information.
    ///
    /// For reference region filters, checks if any stored regions are contained
    /// within the row's genomic region. For motif filters, searches for motif
    /// patterns within the row's sequence.
    ///
    /// # Arguments
    /// * `row` - The data row to check against this filter
    ///
    /// # Returns
    /// * `Result<Option<Vec<ChunkInfo>>, FilterError>` - Vector of match information
    ///   if hits are found, None if no matches, or an error if checking fails
    ///
    /// # Errors
    /// Returns an error if the row lacks required information for the filter type.
    pub(crate) fn hits(&self, row: &Row) -> Result<Option<Vec<MatchedFilterInfo>>, FilterError> {
        match self {
            Filter::ReferenceRegions { regions } => {
                let row_region = row.ref_region()
                    .ok_or(FilterError::NoRegionInTarget)?;

                Ok(regions.self_in_other(row_region))
            }
            Filter::Motifs { motifs } => {
                // Note that if the sequence was not available, it was filled with N, so it
                // will always return None in this case
                Ok(motifs.self_in_other(row.sequence()))
            }
        }
    }

    /// Checks if all elements have the same length
    /// 
    /// # Returns 
    /// Ok if all elements have the same length.
    /// 
    /// # Errors
    /// Returns an error if contained elements lengths are not equal
    pub(crate) fn equal_lengths(&self) -> Option<usize> {
        match self {
            Filter::ReferenceRegions { regions } => {
                regions.equal_len()
            },
            Filter::Motifs { motifs } => {
                motifs.equal_len()
            }
        }
    }
}


/// Information about a filter match within a sequence or region.
///
/// Stores the name of the matched element and its position coordinates.
/// Coordinates use a half-open interval [start, end) where start is inclusive
/// and end is exclusive.
#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct MatchedFilterInfo {
    pub(crate) matched_element_name: String,
    pub(crate) start_index: usize, // Inclusive
    pub(crate) end_index: usize    // Exclusive
}

impl MatchedFilterInfo {
    /// Creates a new ChunkInfo instance.
    ///
    /// # Arguments
    /// * `matched_element_name` - Name/identifier of the matched filter element
    /// * `start_index` - Starting position of the match (inclusive)
    /// * `end_index` - Ending position of the match (exclusive)
    ///
    /// # Returns
    /// * `Self` - The constructed ChunkInfo instance
    pub(crate) fn new(matched_element_name: String, start_index: usize, end_index: usize) -> Self {
        Self { matched_element_name, start_index, end_index }
    }
}