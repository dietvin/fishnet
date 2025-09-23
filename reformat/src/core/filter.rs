use crate::{core::filter::{motifs::Motifs, reference_regions::ReferenceRegions}, error::core::filter::FilterError, execute::config::FilterSource};

pub(crate) mod reference_region;
pub(crate) mod reference_regions;
pub(crate) mod motif;
pub(crate) mod motifs;


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
}