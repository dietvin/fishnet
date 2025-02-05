use std::{collections::HashMap, ops::{Deref, DerefMut}, vec};

use pod5::polars::io::predicates::BatchStats;
use rust_htslib::bam::{self, Read, Record};

#[derive(Debug, thiserror::Error)]
pub enum BamError {
    #[error("HTSLib error: {0}")]
    HTSLibError(#[from] rust_htslib::errors::Error),
    #[error("Could not transform id to String: {0}")]
    IdConversionError(#[from] std::str::Utf8Error),
    #[error("Id not found in index: {0}")]
    IndexError(String),
    #[error("Could not access record: {0}")]
    ValueError(String)
}

#[derive(Debug)]
pub struct BamIndex {
    path: String,
    bam_reader: bam::Reader,
    index: HashMap<String, i64>
}

impl BamIndex {
    pub fn new(path: &str) -> Result<Self, BamError> {
        let mut bam = bam::Reader::from_path(path)?;
        let mut index: HashMap<String, i64> = HashMap::new();

        let mut offset = bam.tell();
        while let Some(read) = bam.records().next() {
            let read = read?;
            let id = std::str::from_utf8(read.qname())?;
            index.insert(String::from(id), offset);

            offset = bam.tell();
        }

        Ok(BamIndex { 
            path: String::from(path), 
            bam_reader: bam, 
            index 
        })
    }

    pub fn get(&mut self, id: &str) -> Result<Record, BamError> {
        let offset = *self.index.get(id).ok_or(BamError::IndexError(String::from(id)))?;

        self.bam_reader.seek(offset)?;
        if let Some(record) = self.bam_reader.records().next() {
            let record = record?;
            Ok(record)
        } else {
            Err(BamError::ValueError(String::from(id)))
        }
    }

    pub fn get_index(&self) -> &HashMap<String, i64> {
        &self.index
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }
}