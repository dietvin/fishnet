use std::collections::HashMap;
use pod5::polars::io::csv::read;
use pod5::polars_arrow::array::Int16Array;
use pod5::{self};
use itertools::multizip;
use std::ops::{Deref, DerefMut, Index, IndexMut};

#[derive(Debug, thiserror::Error)]
pub enum Pod5Error {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Pod5 error: {0}")]
    Pod5Error(#[from] pod5::error::Pod5Error),
    #[error("Polars error: {0}")]
    PolarsError(#[from] pod5::polars::prelude::PolarsError),
}

#[derive(Debug)]
pub struct ReadDataset {
    reads: HashMap<String, Pod5Read>
}

impl Deref for ReadDataset {
    type Target = HashMap<String, Pod5Read>;

    fn deref(&self) -> &Self::Target {
        &self.reads
    }
}

impl DerefMut for ReadDataset {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reads
    }
}

impl<'a> IntoIterator for &'a ReadDataset {
    type Item = (&'a String, &'a Pod5Read);
    type IntoIter = std::collections::hash_map::Iter<'a, String, Pod5Read>;

    fn into_iter(self) -> Self::IntoIter {
        self.reads.iter()
    }
}

impl<'a> IntoIterator for &'a mut ReadDataset {
    type Item = (&'a String, &'a mut Pod5Read);
    type IntoIter = std::collections::hash_map::IterMut<'a, String, Pod5Read>;

    fn into_iter(self) -> Self::IntoIter {
        self.reads.iter_mut()
    }
}

impl ReadDataset {
    /// Parses a pod5 file into a ReadDataset object.
    /// 
    /// This is done in the following steps:
    /// 1. Read the file itself into a Reader (provided by the pod5-rs package; https://github.com/bsaintjo/pod5-rs)
    /// 
    /// 2. Accesses the underlying Polars dataframe containing the metadata (Reads dataframe) and iterate it
    /// row-wise, extracting needed information for each read and storing it in a Hashmap where the keys are
    /// the read IDs in binary form. 
    /// 
    /// 3. Access the underlying Polars dataframe chunks containing the signal themselves. Like before they
    /// are iterated row-wise, extracting the signal stored a given row and adding it to the fitting read.
    /// 
    /// 4. Rework the hashmap to contain the string representations of the read IDs as keys. Then wrap the 
    /// hashmap in a ReadDataset object and return. 
    pub fn from_pod5(path: &str) -> Result<Self, Pod5Error> {
        let file = std::fs::File::open(path)?;
        let mut pod5_reader = pod5::reader::Reader::from_reader(file)?;

        let mut read_collection = HashMap::new();

        for read_df in pod5_reader.read_dfs()?.flatten() {
            let df = read_df
                .parse_read_ids("uuid")?
                .into_inner();
            
            // Extract the columns of the reads df
            // https://stackoverflow.com/questions/72440403/iterate-over-rows-polars-rust
            let objects = df.take_columns();
            // Take only the columns of iterest and zip them together for row-wise iteration
            let read_id_ = objects[0].binary()?.iter();
            let uuid_ = objects[21].str()?.iter();
            let combined = multizip((read_id_, uuid_));

            // Iterate over each row in the DataFrame and insert the wanted row-wise info into the hashmap 
            for (r, u) in combined {
                if let (Some(binary_id), Some(string_id)) = (r, u) {
                    let binary_id = binary_id.to_vec();
                    let string_id = String::from(string_id);
                    let pod5_read = Pod5Read::new(string_id);
                    read_collection.insert(binary_id, pod5_read);
                }
            }
        }

        for signal_df in pod5_reader.signal_dfs()?.flatten() {
            let df = signal_df
                .decompress_signal("signal_decompressed")?
                .into_inner();

            let objects = df.take_columns();
            let read_id_ = objects[0].binary()?.iter();
            let signal_ = objects[3].list()?.iter();
            let combined = multizip((read_id_, signal_));

            for (r, s) in combined {
                // Unwrap the row data
                if let (Some(read_id), Some(signal)) = (r, s) {
                    // Get the Pod5Read from the collection
                    if let Some(pod5_read) = read_collection.get_mut(read_id) {
                        // Transform the list to a native Vector before adding it to the Pod5Read
                        if let Some(array) = signal.as_any().downcast_ref::<Int16Array>() {
                            let mut signal = array.values().to_vec();
                            pod5_read.add_signal_info(&mut signal);
                        }
                    }                    
                }
            }
        }
        
        // Update the keys to the string representations of the read IDs
        let read_collection = read_collection.iter().map(|(_, v)| (String::from(v.get_id()), v.clone())).collect::<HashMap<String, Pod5Read>>();
        Ok(ReadDataset { reads: read_collection })
    }

    /// Returns the number of Pod5Reads stored in the ReadDataset
    pub fn num_reads(&self) -> usize {
        self.reads.len()
    }
}

#[derive(Debug, Clone)]
pub struct Pod5Read {
    read_id: String,
    signal: Vec<i16>,
}

impl Pod5Read {
    /// Create a new instance of a Pod5Read from a read_id and the number of samples value.
    /// The signal gets initialized by an empty vector. The signal itself can then be appended
    /// in the 'add_signal_info' method. 
    fn new(read_id: String) -> Self {
        Pod5Read {
            read_id: read_id,
            signal: vec![]
        }
    }

    /// Add a signal (chunk) to a Pod5Read object. 
    /// 
    /// Important note: Some signals seem to be split into multiple chunks where each chunk is stored in a 
    /// row of the signal dataframe in the pod5 file. The length of these subsets taken together results in 
    /// the number of samples stored in the Reads dataframe. 
    /// 
    /// The method handles these chunks by simply appending the latest chunk (in order of the rows in the df)
    /// to the signal already stored in the Pod5Read. This is assuming that the original order is retained in
    /// the pod5 file. When all chunks are added the length is equal to the num_samples value. 
    fn add_signal_info(&mut self, signal: &mut Vec<i16>) {
        self.signal.append(signal);
    }

    pub fn get_id(&self) -> &str {
        &self.read_id
    }

    /// Returns a reference to the stored signal.
    pub fn get_signal(&self) -> &Vec<i16> {
        &self.signal
    }

    pub fn get_signal_len(&self) -> usize {
        self.signal.len()
    }
}



// Note: Structure of the Reads dataframe:
//     "read_id" -> 0
//     "signal" -> 1
//     "read_number" -> 2
//     "start" -> 3
//     "median_before" -> 4
//     "num_minknow_events" -> 5
//     "tracked_scaling_scale" -> 6
//     "tracked_scaling_shift" -> 7
//     "predicted_scaling_scale" -> 8
//     "predicted_scaling_shift" -> 9
//     "num_reads_since_mux_change" -> 10
//     "time_since_mux_change" -> 11
//     "num_samples" -> 12
//     "channel" -> 13
//     "well" -> 14
//     "pore_type" -> 15
//     "calibration_offset" -> 16
//     "calibration_scale" -> 17
//     "end_reason" -> 18
//     "end_reason_forced" -> 19
//     "run_info" -> 20
//     "uuid" -> 21 (added via the parse_read_ids function)
