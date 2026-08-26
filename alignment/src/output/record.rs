use pod5_reader_api::read::Pod5Read;

use crate::{bam::read::BamRead, error::output::OutputRecordError, output::schema::OutputSchema};

fn pod5_read_into_output_data<S: OutputSchema>(
    pod5_read: Pod5Read,
    shift: f32,
    scale: f32
) -> Result<Option<Vec<f32>>, OutputRecordError> {
    if S::HAS_SIGNAL {
        let signal = pod5_read.into_signal()?;
        return Ok(Some(signal
            .iter()
            .map(|el| (*el as f32 - shift) / scale)
            .collect::<Vec<f32>>()
        ));
    } else {
        return Ok(None)
    }
}

pub trait IntoOutputRecord<S: OutputSchema> {
    fn into_output_record(
        self,
        pod5_read: Pod5Read,
        bam_read: BamRead
    ) -> Result<OutputRecord, OutputRecordError>;
}

pub struct QueryToSignalResult {
    pub query_to_sig: Vec<usize>,
    pub scale: f32,
    pub shift: f32
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

        let signal = pod5_read_into_output_data::<S>(
            pod5_read,
            self.shift,
            self.scale
        )?;

        let query_to_sig = Some(self.query_to_sig);
        let query_shift = Some(self.shift);
        let query_scale = Some(self.scale);
        let ref_to_sig = None;
        let ref_shift = None;
        let ref_scale = None;


        Ok(OutputRecord { 
            read_id,
            query_to_sig, query_shift, query_scale,
            ref_to_sig, ref_shift, ref_scale,
            ref_name,
            ref_start,
            query_seq,
            ref_seq,
            signal
        })
    }
}

pub struct RefToSignalResult {
    pub ref_to_sig: Vec<usize>,
    pub scale: f32,
    pub shift: f32
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

        let signal = pod5_read_into_output_data::<S>(
            pod5_read,
            self.shift,
            self.scale
        )?;

        let query_to_sig = None;
        let query_shift = None;
        let query_scale = None;
        let ref_to_sig = Some(self.ref_to_sig);
        let ref_shift = Some(self.shift);
        let ref_scale = Some(self.scale);

        Ok(OutputRecord { 
            read_id,
            query_to_sig, query_shift, query_scale,
            ref_to_sig, ref_shift, ref_scale,
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
    pub query_shift: f32,
    pub query_scale: f32,
    pub ref_to_sig: Vec<usize>,
    pub ref_shift: f32,
    pub ref_scale: f32,
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

        // Using ref shift / scale for now; need to think about the best solution here...
        let signal = pod5_read_into_output_data::<S>(
            pod5_read,
            self.ref_shift,
            self.ref_scale
        )?;

        let query_to_sig = Some(self.query_to_sig);
        let query_shift = Some(self.query_shift);
        let query_scale = Some(self.query_scale);
        let ref_to_sig = Some(self.ref_to_sig);
        let ref_shift = Some(self.ref_shift);
        let ref_scale = Some(self.ref_scale);
        
        Ok(OutputRecord { 
            read_id,
            query_to_sig, query_shift, query_scale,
            ref_to_sig, ref_shift, ref_scale,
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
    pub query_shift: Option<f32>,
    pub query_scale: Option<f32>,

    pub ref_to_sig: Option<Vec<usize>>,
    pub ref_shift: Option<f32>,
    pub ref_scale: Option<f32>,
    
    pub ref_name: Option<String>,
    pub ref_start: Option<usize>,

    pub query_seq: Option<String>,
    pub ref_seq: Option<String>,

    pub signal: Option<Vec<f32>>
}