use crate::{core::{alignment_loader::Row, filter::{motifs::Motifs, reference_regions::ReferenceRegions}}, error::core::filter::FilterError, execute::config::FilterSource};

pub(crate) mod reference_region;
pub(crate) mod reference_regions;
pub(crate) mod motif;
pub(crate) mod motifs;

#[derive(Debug)]
pub(crate) enum Filter {
    ReferenceRegions { regions: ReferenceRegions },
    Motifs { motifs: Motifs}
}

impl Filter {
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

    pub(crate) fn passes(&self, row: &Row) -> Result<Option<ChunkInfo>, FilterError> {
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
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ChunkInfo {
    pub(crate) matched_element_name: String,
    pub(crate) start_index: usize, // Inclusive
    pub(crate) end_index: usize    // Exclusive
}

impl ChunkInfo {
    pub(crate) fn new(matched_element_name: String, start_index: usize, end_index: usize) -> Self {
        Self { matched_element_name, start_index, end_index }
    }
}