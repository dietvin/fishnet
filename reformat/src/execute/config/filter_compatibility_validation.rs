//! # Filter Compatibility Validation
//!
//! This module handles the complex validation logic for ensuring that the user's chosen
//! filtering options are compatible with the available alignment data and target alignment type.
//!
//! ## Background
//!
//! The alignment input data can contain different combinations of alignment types and sequences,
//! which affects what filtering operations are possible. Users can filter data in two main ways:
//!
//! 1. **Reference-based filtering**: Filter by genomic coordinates (ref-regions, bed-file, positions-of-interest)
//! 2. **Motif-based filtering**: Filter by sequence motifs (motifs, motifs-file)
//!
//! The alignment data can contain:
//! - Query-to-signal alignments (how query sequence maps to raw signal)
//! - Reference-to-signal alignments (how reference sequence maps to raw signal)  
//! - Query sequences
//! - Reference sequences
//! - Raw signal data
//!
//! ## Validation Matrix
//!
//! | Alignment Content | Filter Type | User Selection | Sequence Req. | Valid? | Notes |
//! |-------------------|-------------|----------------|---------------|---------|--------|
//! | Query only | Reference-based | Any | N/A | NO | Can't filter ref positions without ref alignment |
//! | Query only | Motif-based | Any | Query seq | YES | Need query sequence for motif matching |
//! | Reference only | Reference-based | Any | N/A | YES | Perfect match |
//! | Reference only | Motif-based | Any | Ref seq | YES | Need reference sequence for motif matching |
//! | Both | Reference-based | None | N/A | NO | Must specify alignment type |
//! | Both | Reference-based | Query | N/A | NO | Contradictory: want query but filter needs ref |
//! | Both | Reference-based | Reference | N/A | YES | Perfect match |
//! | Both | Motif-based | None | N/A | NO | Must specify alignment type |
//! | Both | Motif-based | Query | Query seq | YES | Need query sequence |
//! | Both | Motif-based | Reference | Ref seq | YES | Need reference sequence |
//!
//! ## Validation Process
//!
//! The validation follows a four-step process:
//!
//! 1. **Determine target alignment**: Resolve what alignment type will actually be used
//! 2. **Validate alignment exists**: Ensure the target alignment is present in the data
//! 3. **Check filter compatibility**: Ensure the filter type works with the target alignment
//! 4. **Validate sequence requirements**: For motif filtering, ensure required sequences exist
use helper::errors::CliError;

use crate::execute::config::{AlignmentContent, AlignmentType, FilterSource};

/// Validates that the chosen filtering options are compatible with the alignment data
/// and user-specified alignment type.
///
/// # Arguments
///
/// * `filter_source` - The filtering method chosen by the user
/// * `alignment_content` - Description of what data is available in the alignment file
/// * `alignment_type` - User's explicit alignment type choice (if any)
///
/// # Returns
///
/// * `Ok(())` if the configuration is valid
/// * `Err(CliError)` with a descriptive error message if incompatible
pub(super) fn validate_filter_compatibility(
    filter_source: &FilterSource,
    alignment_content: &AlignmentContent,
    alignment_type: &AlignmentType
) -> Result<(), CliError> {
    validate_alignment_exists(alignment_content, &alignment_type)?;
    
    validate_filter_alignment_compatibility(filter_source, &alignment_type)?;

    validate_sequence_requirements(filter_source, alignment_content, &alignment_type)?;

    Ok(())
}

/// Validates that the requested alignment type actually exists in the input data.
///
/// This catches cases where a user explicitly requests an alignment type that
/// isn't present in their data file.
///
/// # Example Error Cases
///
/// - User specifies `--alignment-type query` but file only contains reference alignments
/// - User specifies `--alignment-type reference` but file only contains query alignments
fn validate_alignment_exists(
    alignment_content: &AlignmentContent,
    target_alignment: &AlignmentType
) -> Result<(), CliError> {
    let exists = match target_alignment {
        AlignmentType::Query => alignment_content.has_query_alignment,
        AlignmentType::Reference => alignment_content.has_ref_alignment
    };

    if !exists {
        let alignment_name = match target_alignment {
            AlignmentType::Query => "query",
            AlignmentType::Reference => "reference"
        }; 
        return Err(CliError::InvalidArgument(
            "alignment-type".to_string(),
            format!("Requested {} alignment but it's not present in the input file", alignment_name)        
        ));
    }

    Ok(())
}

/// Validates that the chosen filter type is compatible with the target alignment type.
///
/// The key incompatibility is trying to use reference-based filtering (genomic coordinates)
/// with query alignments. Reference-based filters need reference alignments to work because
/// they operate on genomic coordinate space.
///
/// # Compatibility Rules
///
/// - Reference-based filtering (ref-regions, bed-file, positions-of-interest) -> requires Reference alignment
/// - Motif-based filtering (motifs, motifs-file) -> works with either Query or Reference alignment
fn validate_filter_alignment_compatibility(
    filter_source: &FilterSource,
    target_alignment: &AlignmentType
) -> Result<(), CliError> {
    if filter_source.filters_for_ref() && *target_alignment == AlignmentType::Query {
        return Err(CliError::InvalidArgument(
            "filter arguments".to_string(),
            "Cannot use reference-based filtering (ref-regions, bed-file, positions-of-interest) with query alignment. Use motif-based filtering instead".to_string()        
        ));
    }
    Ok(())
}

/// For motif-based filtering, validates that the required sequence data is present.
///
/// Motif filtering needs to search through sequences to find matching patterns.
/// The sequence type required depends on which alignment type is being processed:
///
/// - Query alignment -> needs query sequence
/// - Reference alignment -> needs reference sequence
///
/// # Why This Matters
///
/// Without the appropriate sequence data, motif filtering cannot function because
/// there's no sequence text to search through for the specified motifs.
fn validate_sequence_requirements(
    filter_source: &FilterSource,
    alignment_content: &AlignmentContent,
    target_alignment: &AlignmentType
) -> Result<(), CliError> {
    if !filter_source.filters_for_ref() {
        let (has_sequence, sequence_type) = match target_alignment {
            AlignmentType::Query => (alignment_content.has_query_sequence, "query"),
            AlignmentType::Reference => (alignment_content.has_ref_sequence, "reference")
        };

        if !has_sequence {
            return Err(CliError::InvalidArgument(
                "filter arguments".to_string(),
                format!(
                    "Motif filtering with {} alignment requires {} sequence data, but it's not present", 
                    sequence_type, 
                    sequence_type
                )            
            ));
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_content(
        query_align: bool, 
        ref_align: bool, 
        query_seq: bool, 
        ref_seq: bool
    ) -> AlignmentContent {
        AlignmentContent {
            has_query_alignment: query_align,
            has_ref_alignment: ref_align,
            has_query_sequence: query_seq,
            has_ref_sequence: ref_seq,
            has_signal: true, // Not relevant for these tests
        }
    }


    #[test]
    fn test_validate_filter_alignment_compatibility() {
        let ref_filter = FilterSource::RefRegionFromInput { 
            regions: vec!["chr1:1-100".to_string()] 
        };
        
        // Reference filter + Query alignment -> should error
        assert!(validate_filter_alignment_compatibility(&ref_filter, &AlignmentType::Query).is_err());
        
        // Reference filter + Reference alignment -> should succeed
        assert!(validate_filter_alignment_compatibility(&ref_filter, &AlignmentType::Reference).is_ok());
    }

    #[test]
    fn test_validate_sequence_requirements() {
        let motif_filter = FilterSource::MotifFromInput {
            motifs: vec!["ATCG".to_string()]
        };
        
        // Query alignment but no query sequence -> should error
        let content = create_test_content(true, false, false, false);
        assert!(validate_sequence_requirements(&motif_filter, &content, &AlignmentType::Query).is_err());
        
        // Query alignment with query sequence -> should succeed
        let content = create_test_content(true, false, true, false);
        assert!(validate_sequence_requirements(&motif_filter, &content, &AlignmentType::Query).is_ok());
    }
}