pub mod iterator;
mod signal_table_index;

use std::{
    collections::HashMap, 
    fs::File, 
    io::{
        Read, 
        Seek, 
        SeekFrom
    }, 
    path::PathBuf
};

use arrow2::{
    array::Array,
    chunk::Chunk, 
    io::ipc::read::{
        read_batch, read_file_dictionaries
    }
};
use uuid::Uuid;
use crate::{
    feather_reader::{
        FeatherReader, 
        FeatherReaderError
    }, 
    file::{
        iterator::{
            ReadIterator, 
            ReadIteratorError
        }, 
        signal_table_index::{
            SignalTableIndex, 
            SignalTableIndexError
        }
    }, 
    footer::{
        embedded_content::EmbeddedContentType, 
        Pod5Footer, 
        Pod5FooterError
    }, 
    read::{
        Pod5Read, 
        Pod5ReadError
    }, 
    tables::{
        reads_table::{
            ReadsTable, 
            ReadsTableError
        }, 
        run_info::{
            RunInfo, 
            RunInfoError
        }, 
        signal_table::{
            SignalTable, 
            SignalTableError, 
        }
    } 
};

const EXPECTED_SIGNATURE: [u8; 8] = [139, 80, 79, 68, 13, 10, 26, 10];

/// Representation of a single pod5 file.
/// 
/// Provides access to reads and run information contained in the file.
/// Handles file parsing in multiple steps:
/// 1. Signature verification
/// 2. Footer parsing
/// 3. Run info extraction
/// 4. Reads table parsing
/// 5. Lazy signal data loading
#[derive(Debug)]
pub struct Pod5File {
    path: PathBuf,
    read_ids: Vec<Uuid>,
    reads: HashMap<Uuid, Pod5Read>,
    run_info : RunInfo,
    signal_table_reader: FeatherReader,
    signal_table_index: SignalTableIndex,
    signal_table_batch_size: u64,
    footer: Pod5Footer
}

impl Pod5File {
    /// Initializes a new pod5 file from a given path.
    /// 
    /// # Arguments
    /// * `path` - Path to the pod5 file
    /// 
    /// # Returns
    /// Result containing the initialized Pod5File or an error
    /// 
    /// # Errors
    /// Returns errors for invalid signatures, file access issues, or parsing failures
    pub fn new(path: PathBuf) -> Result<Self, Pod5FileError> {
        let mut file = File::open(&path)?;

        Self::check_signature(&mut file, SeekFrom::Start(0))?;
        Self::check_signature(&mut file, SeekFrom::End(-8))?;

        let footer = Pod5Footer::new(&mut file)?;

        // Parse the run info table and extract the data
        let run_info = Self::parse_run_info_table(&file, &footer)?;
        // Parse the reads table and extract the data into the read_ids vector and the reads hashmap
        let (read_ids, reads, signal_table_index) = Self::parse_reads_table(&file, &footer)?;
        // Initialize the signal table reader without accessing the data at this point
        let mut signal_table_reader = Self::init_signal_table_reader(&file, &footer)?;
        
        // Infer the signal table batch size from the first batch of the signal table
        // This assumes that all batches have the same length (which should be the case
        // for pod5 files generated from the official API)
        let signal_table_batch_size =  signal_table_reader
            .iter_chunks()?
            .next()
            .ok_or(Pod5FileError::SignalTableChunkSizeError)??
            .len() as u64;

        Ok(Pod5File { 
            path, 
            read_ids,
            reads,
            run_info,
            signal_table_reader,
            signal_table_index,
            signal_table_batch_size,
            footer
        })
    } 

    /// Checks the pod5 file signature at the specified position.
    /// 
    /// The signature must match the expected POD5 file signature.
    /// 
    /// # Arguments
    /// * `file` - File handle to check
    /// * `start` - Position to check (start or end of file)
    /// 
    /// # Errors
    /// Returns InvalidSignature error if the signature doesn't match
    fn check_signature(file: &mut File, start: SeekFrom) -> Result<(), Pod5FileError> {
        let mut start_signature = [0u8; 8];
        file.seek(start)?;

        file.read(&mut start_signature)?;

        if start_signature == EXPECTED_SIGNATURE {
            Ok(())
        } else {
            Err(Pod5FileError::InvalidSignature(start_signature.to_vec(), start))
        }
    }

    /// Parses the run info table from the file.
    /// 
    /// # Arguments
    /// * `file` - File handle
    /// * `footer` - Parsed footer information
    /// 
    /// # Returns
    /// Result containing the parsed RunInfo or an error
    fn parse_run_info_table(file: &File, footer: &Pod5Footer) -> Result<RunInfo, Pod5FileError> {
        let embedded_file_run_info = footer.retrieve_embedded_file(EmbeddedContentType::Unknown)?;
        let mut reader_run_info = FeatherReader::new(
            file.try_clone()?, 
            embedded_file_run_info.offset(), 
            embedded_file_run_info.length()
        )?;

        let chunk = reader_run_info.get_chunk(0)?;
        let run_info = RunInfo::from_arrow_chunk(chunk)?;
        Ok(run_info)
    }


    /// Parses the reads table from the file.
    /// 
    /// # Arguments
    /// * `file` - File handle
    /// * `footer` - Parsed footer information
    /// 
    /// # Returns
    /// Tuple containing (read_ids, reads) or an error
    fn parse_reads_table(file: &File, footer: &Pod5Footer) -> Result<(Vec<Uuid>, HashMap<Uuid, Pod5Read>, SignalTableIndex), Pod5FileError> {
        let embedded_file_reads_table = footer.retrieve_embedded_file(EmbeddedContentType::ReadsTable)?;
        let mut reader_reads_table = FeatherReader::new(
            file.try_clone()?, 
            embedded_file_reads_table.offset(), 
            embedded_file_reads_table.length()
        )?;

        let mut read_ids = Vec::new();
        let mut reads = HashMap::new();
        let mut n_signal_table_rows: usize = 0;

        for chunk_res in reader_reads_table.iter_chunks()? {
            let chunk = chunk_res?;
            let reads_table = ReadsTable::from_chunk(chunk)?;
    
            for read_res in reads_table {
                let read = read_res?;
                let read_id = read.read_id();
                let signal_indices = read.signal_indices();

                // Determine the number of rows in the signal table (corresponds to the highest index found in the signal indices)
                if let Some(&val) = signal_indices.iter().max() {
                    let val = val as usize;
                    if val > n_signal_table_rows {
                        n_signal_table_rows = val;
                    }
                }

                read_ids.push(read_id.clone());
                reads.insert(read_id.clone(), read);
            }
        }
        // The max value is a 0-based index, to get the length add 1
        n_signal_table_rows += 1;

        let signal_table_index = SignalTableIndex::new(&reads, n_signal_table_rows)?;

        Ok((read_ids, reads, signal_table_index))
    }

    /// Initializes the signal table reader.
    /// 
    /// # Arguments
    /// * `file` - File handle
    /// * `footer` - Parsed footer information
    /// 
    /// # Returns
    /// Result containing the FeatherReader for the signal table or an error
    fn init_signal_table_reader(file: &File, footer: &Pod5Footer) -> Result<FeatherReader, Pod5FileError> {
        let embedded_file_signal_table = footer.retrieve_embedded_file(EmbeddedContentType::SignalTable)?;
        
        Ok(FeatherReader::new(
            file.try_clone()?, 
            embedded_file_signal_table.offset(), 
            embedded_file_signal_table.length()
        )?)
    }

    /// Returns the path to the pod5 file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns reference to the run info.
    pub fn run_info(&self) -> &RunInfo {
        &self.run_info
    }

    /// Returns reference to the list of read IDs.
    pub fn read_ids(&self) -> &Vec<Uuid> {
        &self.read_ids
    }

    /// Returns the number of reads in the file.
    pub fn n_reads(&self) -> usize {
        self.read_ids.len()
    }

    /// Returns reference to the reads HashMap.
    pub(crate) fn reads(&self) -> &HashMap<Uuid, Pod5Read> {
        &self.reads
    }

    /// Returns reference to the footer.
    pub(crate) fn footer(&self) -> &Pod5Footer {
        &self.footer
    }

    /// Gets a read by its UUID, including signal data.
    /// 
    /// # Arguments
    /// * `read_id` - UUID of the read to retrieve
    /// 
    /// # Returns
    /// Result containing the complete Pod5Read or an error
    /// 
    /// # Errors
    /// Returns errors for missing reads or signal reconstruction failures
    pub fn get(&mut self, read_id: &Uuid) -> Result<Pod5Read, Pod5FileError> {
        let mut read = self.reads.get(read_id)
            .ok_or(Pod5FileError::ReadNotFound(read_id.clone()))?
            .clone();

        if read.signal().is_some() {
            return Ok(read);
        }

        let mut signal = Vec::new();
        let mut sample_count = 0;

        let chunk_indices = read
            .signal_indices()
            .iter()
            .map(|idx| {ChunkRowIndex { 
                chunk: (idx / self.signal_table_batch_size) as usize,
                row: (idx % self.signal_table_batch_size) as usize
            }})
            .collect::<Vec<ChunkRowIndex>>();

        let mut current_signal_table_idx = chunk_indices[0].chunk;
        let mut signal_table = SignalTable::from_chunk(
            self.get_signal_table_chunk(current_signal_table_idx)?
        )?;

        for chunk_index in chunk_indices {
            if chunk_index.chunk != current_signal_table_idx {
                // Load a new chunk only if needed
                current_signal_table_idx = chunk_index.chunk;
                signal_table = SignalTable::from_chunk(
                    self.get_signal_table_chunk(current_signal_table_idx)?
                )?;
            }

            let mut signal_table_row = signal_table.get(chunk_index.row)?;

            if signal_table_row.read_id != *read_id {
                return Err(Pod5FileError::SignalReconstructIdError(
                    signal_table_row.read_id,
                    read_id.clone()
                ));
            }

            signal.append(&mut signal_table_row.signal);
            sample_count += signal_table_row.sample_count;
        }

        if sample_count != (read.require_num_samples()? as usize) {
            return Err(Pod5FileError::SignalReconstructLengthError(
                sample_count, 
                read.require_num_samples()? as usize
            ));
        }

        read.set_signal(signal);

        Ok(read)
    }

    /// Gets a specific chunk from the signal table.
    /// 
    /// # Arguments
    /// * `chunk_index` - Index of the chunk to retrieve
    /// 
    /// # Returns
    /// Result containing the chunk data or an error
    fn get_signal_table_chunk(&mut self, chunk_index: usize) -> Result<Chunk<Box<dyn Array>>, Pod5FileError> {
        let metadata = self.signal_table_reader.metadata().clone();
        let reader = self.signal_table_reader
            .embedded_reader_mut();
        let dictionaries = read_file_dictionaries(
            reader, 
            &metadata, 
            &mut Default::default()
        )?;
        Ok(
            read_batch(
                reader, 
                &dictionaries, 
                &metadata, 
                None, 
                None, 
                chunk_index, 
                &mut Default::default(), 
                &mut Default::default()
            )?
        )
    }

    pub(crate) fn signal_table_index(&self) -> &SignalTableIndex {
        &self.signal_table_index
    }

    /// Returns mutable reference to the signal table reader.
    pub(crate) fn signal_table_reader_mut(&mut self) -> &mut FeatherReader {
        &mut self.signal_table_reader
    }

    /// Creates an iterator over all reads in the file.
    /// 
    /// # Returns
    /// Result containing the ReadIterator or an error
    pub fn iter_reads(&mut self) -> Result<ReadIterator<'_>, Pod5FileError> {
        Ok(ReadIterator::new(self)?)
    }
}

/// Helper struct representing a row index within a specific chunk of the signal table.
struct ChunkRowIndex {
    chunk: usize,
    row: usize
}

/// Enum representing all possible errors that can occur when working with a Pod5File.
#[derive(Debug, thiserror::Error)]
pub enum Pod5FileError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid signature: {0:?} (start: {1:?})")]
    InvalidSignature(Vec<u8>, SeekFrom),
    #[error("Footer error: {0}")]
    FooterError(#[from] Pod5FooterError),
    #[error("Feather reader error: {0}")]
    EmbeddedFileError(#[from] FeatherReaderError),
    #[error("Arrow2 error: {0}")]
    Arrow2Error(#[from] arrow2::error::Error),
    #[error("Run info error: {0}")]
    RunInfoError(#[from] RunInfoError),
    #[error("Pod5Read error: {0}")]
    ReadError(#[from] Pod5ReadError),
    #[error("Reads table error: {0}")]
    ReadsTableError(#[from] ReadsTableError),
    #[error("Signal table error: {0}")]
    SignalTableError(#[from] SignalTableError),
    #[error("Read not found: {0}")]
    ReadNotFound(Uuid),
    #[error("Id mismatch for signal chunk: {0} vs. {1} (expected)")]
    SignalReconstructIdError(Uuid, Uuid),
    #[error("Signal length mismatch: {0} vs. {1} (expected)")]
    SignalReconstructLengthError(usize, usize),
    #[error("Read iterator error: {0}")]
    ReadIteratorError(#[from] ReadIteratorError),
    #[error("Signal table index error: {0}")]
    SignalTableIndexError(#[from] SignalTableIndexError),
    #[error("Could not determine the chunk size for the signal table")]
    SignalTableChunkSizeError
}