use std::{fs::File, io::{BufRead, BufReader}, path::PathBuf};

use crate::{core::filter::motif::Motif, error::core::filter::MotifsError, execute::config::FilterSource};

pub(crate) struct Motifs {
    motifs: Vec<Motif>
}

impl Motifs {
    pub(crate) fn from_filter_source(filter_source: &FilterSource) -> Result<Self, MotifsError> {
        match filter_source {
            FilterSource::MotifFromFile { path } => Self::from_fasta(path),
            FilterSource::MotifFromInput { motifs } => Self::from_motifs(motifs),
            _ => return Err(MotifsError::InvalidFilterSource)
        }
    }

    fn from_fasta(path: &PathBuf) -> Result<Self, MotifsError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let mut motifs = Vec::new();
        let mut current_name: Option<String> = None;
        let mut current_seq = String::new();
    
        for line_res in reader.lines() {
            let line = line_res?;
            let line = line.trim();
    
            if line.starts_with('>') {
                // flush previous motif
                if let Some(name) = current_name.take() {
                    let motif = Motif::new(&name, &current_seq)?;
                    motifs.push(motif);
                }
    
                // start new motif
                current_name = Some(line[1..].to_string());
                current_seq.clear();
            } else if !line.is_empty() {
                current_seq.push_str(line);
            }
        }
    
        // flush last motif
        if let Some(name) = current_name {
            let motif = Motif::new(&name, &current_seq)?;
            motifs.push(motif);
        }
    
        Ok(Self { motifs })
    }

    fn from_motifs(motifs: &Vec<String>) -> Result<Self, MotifsError> {
        let motifs = motifs.iter()
            .enumerate()
            .map(|(idx, m)| {
                let name = format!("motif{}", idx);
                Motif::new(&name, m).map_err(|e| MotifsError::MotifError(e))
            }) 
            .collect::<Result<Vec<Motif>, MotifsError>>()?;

        Ok(Self { motifs })
    }

    pub(crate) fn contains(&self, other: &str) -> Option<String> {
        for motif in &self.motifs {
            if motif.is_in(other) {
                return Some(motif.name().to_string());
            }
        }
        None
    }
}