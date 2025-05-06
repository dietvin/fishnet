use std::path::PathBuf;
use rust_htslib::bam::{record::{Aux, AuxArray}, Header, Read, Reader, Record, Writer};
use crate::{
    core::refinement::signal_map_refiner::SigMapRefiner, 
    error::output_errors::OutputError,
    cli::args_to_input::WhichToAlign
};

pub struct BamWriter {
    writer: Writer
}

impl BamWriter {
    pub fn new(path: &PathBuf, source_path: &PathBuf) -> Result<Self, OutputError> {
        let source_bam = Reader::from_path(source_path)?;
        let header = Header::from_template(source_bam.header());
        let writer = Writer::from_path(
            path, 
            &header, 
            rust_htslib::bam::Format::Bam
        )?;

        Ok(BamWriter { 
            writer: writer
        })
    }

    pub fn write_read(&mut self, sig_map_refiner: &mut SigMapRefiner, which_to_align: &WhichToAlign) -> Result<(), OutputError> {
        let query_to_signal = if *which_to_align == WhichToAlign::Both || *which_to_align == WhichToAlign::Query {
            Some(
                sig_map_refiner.refined_query_to_sig()?
                    .iter()
                    .map(|&el| el as u32)
                    .collect::<Vec<u32>>()
            )
        } else {
            None
        };

        let reference_to_signal = if *which_to_align == WhichToAlign::Both || *which_to_align == WhichToAlign::Reference {
            Some(
                sig_map_refiner.refined_ref_to_sig()?
                    .iter()
                    .map(|&el| el as u32)
                    .collect::<Vec<u32>>()
            )
        } else {
            None
        };
        
        let record = sig_map_refiner.bam_record_mut();

        if let Some(alignment) = query_to_signal {
            self.add_tag(record, "QS", alignment)?;
        }
        if let Some(alignment) = reference_to_signal {
            self.add_tag(record, "RS", alignment)?;
        }

        self.writer.write(record)?;
        Ok(())
    }

    fn add_tag(&self, record: &mut Record, tag: &str, alignment: Vec<u32>) -> Result<(), OutputError> {
        let aux_array: AuxArray<u32> = alignment
            .as_slice()
            .into();
        let tag_data = Aux::ArrayU32(aux_array);
        record.push_aux(tag.as_bytes(), tag_data)?;
        Ok(())
    }
}