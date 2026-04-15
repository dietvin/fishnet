/*
* This module contains the logic for determining which embedded kmer table fits to the data by parsing the
* basecall model name from the BAM header. 
*/

use bstr::ByteSlice;
use kmer_table::{
    kmer_table::{KmerSource, KmerTable},
    kmer_table_data::KmerTableData
};
use noodles::sam::Header;

use crate::error::execute::KmerTableLoadingError;

/// Main function to load the kmer table for given data.
/// 
/// Parses the basecall model name from the header, initializes a fitting AvailableKmerTable
/// from it and transforms it into the KmerTable.
/// 
/// # Arguments
/// * `header` - The BAM file header
/// 
/// # Returns
/// * `Ok(KmerTable)` - The kmer table fitting to the basecalled data
/// 
/// # Errors
/// * `KmerTableLoadingError::InconsistentBasecallModel` - If `DS` tags from different
///     read groups contain different basecall_model values
/// * `KmerTableLoadingError::BasecallModelNotFound` - If no basecall_model value is 
///     found in the header
/// * `KmerTableLoadingError::UnfittingBasecallModel` - If no kmer table can be assigned to
///     the given basecall model name
/// * `KmerTableLoadingError::DeserializationError` - If the deserialization fails
pub(crate) fn load_kmer_table(header: &Header) -> Result<KmerTable, KmerTableLoadingError> {
    let basecall_model_str = parse_basecall_model(header)?;
    let fitting_kmer_table = AvailableKmerTables::from_basecall_model_str(basecall_model_str)?;
    let kmer_table = fitting_kmer_table.load()?;

    log::info!("Loaded embedded kmer table '{}' to match basecall model '{}'", kmer_table.source_str(), basecall_model_str);
    
    Ok(kmer_table)
}

const DS_TAG: &[u8; 2] = b"DS";

/// Parse basecall model name from BAM header.
/// 
/// Iterates all read groups in a given BAM header, and checks it contains the 
/// `DS` tag. If so, it attempts to parse the `basecall_model` value from it.
/// 
/// # Arguments
/// * `header` - BAM header to parse
/// 
/// # Returns
/// * `Ok(&str)` - The name of the basecall_model
/// 
/// # Errors
/// * `KmerTableLoadingError::InconsistentBasecallModel` - If `DS` tags from different
///     read groups contain different basecall_model values
/// * `KmerTableLoadingError::BasecallModelNotFound` - If no basecall_model value is 
///     found in the header
fn parse_basecall_model(header: &Header) -> Result<&str, KmerTableLoadingError> {
    let read_groups = header.read_groups();
    log::debug!("Found {} read groups in header", read_groups.len());

    let mut basecall_model: Option<&str> = None;

    for (_, read_group) in read_groups {
        for (tag_name, tag_value) in read_group.other_fields() {
            if tag_name.as_ref() == DS_TAG {
                let mut model_opt = None;
                if let Ok(tag_value_str) = tag_value.to_str() {
                    for el in tag_value_str.split(" ") {
                        if el.starts_with("basecall_model") {
                            model_opt = el.strip_prefix("basecall_model=");
                        }
                    }
    
                    if let Some(model) = model_opt {
                        if let Some(prev_model) = basecall_model {
                            if model != prev_model {
                                return Err(KmerTableLoadingError::InconsistentBasecallModel(
                                    prev_model.to_string(), 
                                    model.to_string()
                                ));
                            }
                        } else {
                            basecall_model = Some(model);
                        }
                    }
                }
                
            }
        }    
    }

    basecall_model.ok_or(KmerTableLoadingError::BasecallModelNotFound)
}


static BYTES_DNA_R10_260: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kmer_table_data_dna_r10_260bps.bin"));
static BYTES_DNA_R10_400: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kmer_table_data_dna_r10_400bps.bin"));
static BYTES_RNA002: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kmer_table_data_rna002.bin"));
static BYTES_RNA004: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kmer_table_data_rna004.bin"));
static BYTES_DNA_R9_450: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kmer_table_data_dna_r9_450bps.bin"));

/// Contains the kmer tables that are embedded
enum AvailableKmerTables {
    /// Expected standardized levels for DNA R10 260bps data
    DnaR10Bps260,
    /// Expected standardized levels for DNA R10 400bps data
    DnaR10Bps400,
    /// Expected standardized levels for DNA R9 data (legacy)
    DnaR9Bps450,
    /// Expected standardized levels for RNA004 data
    RNA004,
    /// Expected standardized levels for RNA002 data (legacy)
    RNA002,
}

impl AvailableKmerTables {
    /// Initializes an AvailableKmerTables instance from a basecalling model name.
    /// 
    /// Matches a model name to the embedded kmer tables by splitting the `_`-separated
    /// elements in the name and matching elements to corresponding indicators.
    /// 
    /// # Arguments
    /// * `model` - Basecalling model name extracted from BAM header
    /// 
    /// # Returns
    /// * Ok(AvailableKmerTables) - A fitting kmer table
    /// 
    /// # Error 
    /// * `KmerTableLoadingError::UnfittingBasecallModel` - If no kmer table can be assigned to
    ///     the given basecall model name
    fn from_basecall_model_str(model: &str) -> Result<Self, KmerTableLoadingError> {
        let main = model.split("@").next().unwrap_or(model);
        let parts: Vec<&str> = main.split("_").collect();

        match parts.as_slice() {
            ["rna002", ..] => Ok(Self::RNA002),
            ["rna004", ..] => Ok(Self::RNA004),
            ["dna", pore, ..] if pore.starts_with("r9.4") => Ok(Self::DnaR9Bps450),
            ["dna", "r10.4.1", ..] if parts.contains(&"260bps") => Ok(Self::DnaR10Bps260),
            ["dna", "r10.4.1", ..] if parts.contains(&"400bps") => Ok(Self::DnaR10Bps400),
            _ => Err(KmerTableLoadingError::UnfittingBasecallModel(model.to_string()))
        }
    }

    /// Load the data for a AvailableKmerTables instance.
    /// 
    /// Deserializes the fitting embedded bytes into the KmerTableData and transforms
    /// it into a KmerTable instance with the source attribute set to Embedded.
    /// 
    /// # Returns
    /// * `Ok(KmerTable)` - A kmer table fitting to a given option
    /// 
    /// # Errors
    /// * `KmerTableLoadingError::DeserializationError` - If the deserialization fails
    fn load(&self) -> Result<KmerTable, KmerTableLoadingError> {
        let (name, bytes) = match self {
            Self::DnaR10Bps260 => ("DNA_R10_260bps", BYTES_DNA_R10_260),
            Self::DnaR10Bps400 => ("DNA_R10_400bps", BYTES_DNA_R10_400),
            Self::DnaR9Bps450 => ("DNA_R9_450bps", BYTES_DNA_R9_450),
            Self::RNA004 => ("RNA004", BYTES_RNA004),
            Self::RNA002 => ("RNA002", BYTES_RNA002),
        };

        let kmer_table_data: KmerTableData = bincode::deserialize(bytes)?;
        let kmer_table = kmer_table_data.into_kmer_table(KmerSource::Embedded(name));
        Ok(kmer_table)
    }
}