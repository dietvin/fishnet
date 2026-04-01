use pod5_reader_api::read::Pod5Read;

use crate::{bam::read::BamRead, error::output::OutputRecordError, output::schema::OutputSchema};

pub trait IntoOutputRecord<S: OutputSchema> {
    fn into_output_record(
        self,
        pod5_read: Pod5Read,
        bam_read: BamRead
    ) -> Result<OutputRecord, OutputRecordError>;
}

pub struct QueryToSignalResult {
    pub query_to_sig: Vec<usize>
}

impl<'a, S: OutputSchema> IntoOutputRecord<S> for QueryToSignalResult {
    fn into_output_record(
        self,
        pod5_read: Pod5Read,
        bam_read: BamRead
    ) -> Result<OutputRecord, OutputRecordError> {
        let (
            read_id,
            ref_name,
            ref_start,
            query_seq,
            ref_seq
        ) = bam_read.into_output_data::<S>()?;

        let signal = Some(pod5_read.into_output_data()?);

        let query_to_sig = Some(self.query_to_sig);
        let ref_to_sig = None;

        Ok(OutputRecord { 
            read_id,
            query_to_sig,
            ref_to_sig,
            ref_name,
            ref_start,
            query_seq,
            ref_seq,
            signal
        })
    }
}

pub struct RefToSignalResult {
    pub ref_to_sig: Vec<usize>
}

impl<'a, S: OutputSchema> IntoOutputRecord<S> for RefToSignalResult {
    fn into_output_record(
        self,
        pod5_read: Pod5Read,
        bam_read: BamRead
    ) -> Result<OutputRecord, OutputRecordError> {
        let (
            read_id,
            ref_name,
            ref_start,
            query_seq,
            ref_seq
        ) = bam_read.into_output_data::<S>()?;

        let signal = Some(pod5_read.into_output_data()?);

        let query_to_sig = None;
        let ref_to_sig = Some(self.ref_to_sig);

        Ok(OutputRecord { 
            read_id,
            query_to_sig,
            ref_to_sig,
            ref_name,
            ref_start,
            query_seq,
            ref_seq,
            signal
        })
    }
}


pub struct BothResult {
    pub query_to_sig: Vec<usize>,
    pub ref_to_sig: Vec<usize>
}

impl<'a, S: OutputSchema> IntoOutputRecord<S> for BothResult {
    fn into_output_record(
        self,
        pod5_read: Pod5Read,
        bam_read: BamRead
    ) -> Result<OutputRecord, OutputRecordError> {
        let (
            read_id,
            ref_name,
            ref_start,
            query_seq,
            ref_seq
        ) = bam_read.into_output_data::<S>()?;

        let signal = Some(pod5_read.into_output_data()?);

        let query_to_sig = Some(self.query_to_sig);
        let ref_to_sig = Some(self.ref_to_sig);
        
        Ok(OutputRecord { 
            read_id,
            query_to_sig,
            ref_to_sig,
            ref_name,
            ref_start,
            query_seq,
            ref_seq,
            signal
        })
    }
}

pub struct OutputRecord {
    pub read_id: String,
    
    pub query_to_sig: Option<Vec<usize>>,
    pub ref_to_sig: Option<Vec<usize>>,
    
    pub ref_name: Option<String>,
    pub ref_start: Option<usize>,

    pub query_seq: Option<String>,
    pub ref_seq: Option<String>,

    pub signal: Option<Vec<i16>>
}