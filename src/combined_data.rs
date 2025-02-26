// use pod5::polars::prelude::AllowedOptimizations;
// use rust_htslib::bam::{ext::BamRecordExtensions, record::Cigar};
// use interp::{interp_slice, InterpMode};
// use super::loader::pod5_io;
// use std::io::{self, Write};

// #[derive(Debug, thiserror::Error)]
// pub enum AlignmentError {
//     #[error("Could not convert binary to string: {0}")]
//     Utf8ConversionError(#[from] std::str::Utf8Error),
//     #[error("Bam record is unmapped")]
//     BamRecordUnmapped,
//     #[error("Read IDs do not match (pod5: {0}; bam: {1})")]
//     IDMismatch(String, String),
//     #[error("HTSLib error: {0}")]
//     HTSLibError(#[from] rust_htslib::errors::Error),
//     #[error("Could not convert tag: {0}")]
//     ConversionError(String),
//     #[error("Failed to align query to signal: {0}")]
//     QueryToSignalError(String),
//     #[error("Query to signal alignment not found")]
//     QueryToSignalNotFound,
//     #[error("Failed to align reference to signal: {0}")]
//     RefToSignalError(String),
//     #[error("Ref to signal alignment not found")]
//     RefToSignalNotFound,
// }

// pub struct CombinedRead<'a> {
//     read_id: String,
//     pod5_read: &'a pod5_io::Pod5Read,
//     bam_read: &'a rust_htslib::bam::Record,
//     reverse_signal: bool,
//     query_to_signal: Option<Vec<i64>>,
//     ref_to_signal: Option<Vec<i64>>
// }

// impl<'a> CombinedRead<'a> {
//     /// Initialize a CombinedRead instance from References to a Pod5Read and a Bam Record.
//     /// The alignments are initialized as None. To calculate them, first run `align_to_query` and then
//     /// `align_to_reference`.
//     /// 
//     /// # Arguments 
//     /// * `pod5` - Reference to a Pod5Read
//     /// * `bam` - Reference to a Bam Record
//     /// * `reverse_signal` - Bool indicating if the signal is in 3'->5' direction (e.g. for dRNA reads)
//     pub fn from_pod5_and_bam_record(
//         pod5: &'a pod5_io::Pod5Read, 
//         bam: &'a rust_htslib::bam::Record,
//         reverse_signal: bool
//     ) -> Result<Self, AlignmentError> {    

//         // Check if the read ids match between the pod5 and bam records
//         let pod5_id = pod5.get_id();
//         let bam_id = std::str::from_utf8(bam.qname())?;
//         if pod5_id != bam_id {
//             return Err(AlignmentError::IDMismatch(String::from(pod5_id), String::from(bam_id)));
//         }

//         // Check if the read is mapped
//         if bam.is_unmapped() {
//             return Err(AlignmentError::BamRecordUnmapped);
//         }

//         Ok(CombinedRead {
//             read_id: String::from(pod5.get_id()),
//             pod5_read: pod5,
//             bam_read: bam,
//             reverse_signal,
//             query_to_signal: None,
//             ref_to_signal: None
//         })
//     }

//     /// Align the query (base-called) sequence to the signal. If successfull the alignment is stored in
//     /// the query_to_signal member variable.
//     pub fn align_to_query(&mut self) -> Result<(), AlignmentError> {      
//         let trimmed_signal = self.trim_signal();

//         let mv_table = self.extract_vec_tag_value(b"mv")?;
//         let stride = mv_table[0] as i64;
//         let mv_table = mv_table[1..].to_vec();
        
//         let signal_len = trimmed_signal.len() as i64;

//         let mut query_to_signal = vec![];
//         for (i, mv) in mv_table.iter().enumerate() {
//             if *mv == 1i16 {
//                 query_to_signal.push((i as i64)*stride);
//             }
//         }
//         query_to_signal.push(signal_len);
        
//         if self.reverse_signal {
//             query_to_signal = query_to_signal.iter().rev().map(|el| signal_len - *el).collect();
//         }

//         if query_to_signal.len()-1 != self.bam_read.seq_len() {
//             return Err(AlignmentError::QueryToSignalError(format!("{}: Move table discordant with basecalls", self.read_id)));
//         } else if mv_table.len() != (signal_len/stride) as usize {
//             return Err(AlignmentError::QueryToSignalError(format!("{}: Move table discordant with signal length", self.read_id)));
//         }
        
//         self.query_to_signal = Some(query_to_signal);
//         Ok(())
//     }

//     /// Trims the signal according to the `sp`, `ts` and `ns` split read tags 
//     /// (more information on the [Split read tags](https://github.com/nanoporetech/dorado/blob/release-v0.9/documentation/SAM.md)).
//     fn trim_signal(&self) -> Vec<i16> {
//         let sp = self.extract_u_tag_value(b"sp", 0);
//         let ts = self.extract_u_tag_value(b"ts", 0);
//         let ns = self.extract_u_tag_value(b"ns", self.pod5_read.get_signal().len());

//         let signal = self.pod5_read.get_signal();
//         if self.reverse_signal {
//             let signal = CombinedRead::reverse_signal(signal);
//             let signal_trimmed = signal[sp+ts..ns].to_vec();
//             let signal = CombinedRead::reverse_signal(&signal_trimmed);
//             signal
//         } else {
//             signal[sp+ts..ns].to_vec()
//         }
//     }

//     /// Extract a tag from the BAM record. The tag must be of type Aux::U8, Aux::U16 or Aux::U32. 
//     /// Returns the provided default value if either the tag can not be found or the type of the 
//     /// tag does not match.
//     /// 
//     /// Function is intended for extracting the `sp`, `ts` and `ns` tags.
//     /// 
//     /// # Arguments
//     /// * `tag` - The binary representation of the tag as stored in the bam read
//     /// * `default` - Default value that gets returned in case the tag is not an unsigned integer
//     fn extract_u_tag_value(&self, tag: &[u8], default: usize) -> usize {
//         match self.bam_read.aux(tag) {
//             Ok(rust_htslib::bam::record::Aux::U8(value)) => value as usize,
//             Ok(rust_htslib::bam::record::Aux::U16(value)) => value as usize,
//             Ok(rust_htslib::bam::record::Aux::U32(value)) => value as usize,
//             _ => default
//         }
//     }

//     /// Reverses a given signal vector
//     /// 
//     /// # Arguments
//     /// * `signal` - Vector of the current measurements
//     fn reverse_signal(signal: &Vec<i16>) -> Vec<i16> {
//         signal.iter().rev().map(|el| *el).collect::<Vec<i16>>()
//     }

//     /// Extract a tag from the BAM record. The tag must be of type Aux::ArrayI8, Aux::ArrayI16 or
//     /// Aux::ArrayI32. If the type matches, the array gets converted to a Vector of i16. 
//     /// Returns an HTSLibError if the tag is not found, and a ConversionError if the tag can not be 
//     /// converted.
//     /// 
//     /// Function is intended for extracting the move table (`mv` tag).
//     /// 
//     /// # Arguments 
//     /// * `tag` - The binary representation of the tag as stored in the bam read
//     fn extract_vec_tag_value(&self, tag: &[u8]) -> Result<Vec<i16>, AlignmentError> {
//         match self.bam_read.aux(tag) {
//             Ok(rust_htslib::bam::record::Aux::ArrayI8(vec)) => Ok(vec.iter().map(|el| el as i16).collect::<Vec<i16>>()),
//             Ok(rust_htslib::bam::record::Aux::ArrayI16(vec)) => Ok(vec.iter().map(|el| el as i16).collect::<Vec<i16>>()),
//             Ok(rust_htslib::bam::record::Aux::ArrayI32(vec)) => Ok(vec.iter().map(|el| el as i16).collect::<Vec<i16>>()),
//             Err(e) => Err(AlignmentError::HTSLibError(e)),
//             _ => Err(AlignmentError::ConversionError(format!("Could not convert tag {:?} to Vec<i16>", tag)))
//         }
//     }

//     /// Get a reference to the query to signal alignment if it has been calculated. 
//     /// Returns a QueryToSignalError otherwise.
//     pub fn get_query_to_signal(&self) -> Result<&Vec<i64>, AlignmentError> {
//         match self.query_to_signal {
//             Some(ref qts) => Ok(qts),
//             None => Err(AlignmentError::QueryToSignalNotFound)
//         }
//     }


//     /// Align the reference sequence to the signal. If successful bind the alignment to the 
//     /// `ref_to_signal` member variable.
//     pub fn align_to_reference(&mut self) -> Result<(), AlignmentError> {
//         if self.query_to_signal == None {
//             return Err(AlignmentError::QueryToSignalNotFound);
//         }

//         let mut cigar = self.bam_read
//             .cigar()
//             .iter()
//             .map(|el| *el)
//             .collect::<Vec<Cigar>>();
        
//         if self.bam_read.is_reverse() {
//             cigar = cigar.iter().rev().map(|el| *el).collect();
//         }

//         // Non-match operations at the end of the cigar strings must be cut off
//         // Determine the number of these operations and remove them from the cigar vector. 
//         let mut cutoff_len = 0;
//         for (idx, el) in cigar.iter().rev().enumerate() {
//             if CombinedRead::is_match_ops(el) {
//                 cutoff_len = idx;
//                 break;
//             }
//         }
//         if cutoff_len >= cigar.len() {
//             return Err(AlignmentError::RefToSignalError(
//                 format!("{}: No match operations found in alignment cigar", self.pod5_read.get_id())
//             ));
//         }
//         cigar.truncate(cigar.len()-cutoff_len);

//         let mut file = std::fs::File::create(format!("/home/vincent/projects/resquiggle_tool/rustmora/understanding_knots/{}_cigar.txt", self.read_id)).unwrap();
//         for el in &cigar {
//             let cigar_string = el.char();
//             let len = el.len();
//             writeln!(file, "{}, {}", cigar_string, len);
//         }


//         // Calculate the knots 
//         let ref_knots = self.calculate_knots(&cigar, &CombinedRead::consumes_reference);
//         let query_knots = self.calculate_knots(&cigar, &CombinedRead::consumes_query);
//         // let (ref_knots, query_knots) = self.calculate_knots(&cigar);
        
//         let mut file = std::fs::File::create(format!("/home/vincent/projects/resquiggle_tool/rustmora/understanding_knots/{}_ref_knots.txt", self.read_id)).unwrap();
//         for el in &ref_knots {
//             writeln!(file, "{}", el);
//         }
        
//         let mut file = std::fs::File::create(format!("/home/vincent/projects/resquiggle_tool/rustmora/understanding_knots/{}_query_knots.txt", self.read_id)).unwrap();
//         for el in &query_knots {
//             writeln!(file, "{}", el);
//         }


//         let last_el = ref_knots[ref_knots.len()-1];
//         let mut interp_vals = Vec::with_capacity((last_el as usize)+1);
//         for i in 0..last_el+1 {
//             interp_vals.push(i as f64);
//         }

//         let ref_to_read_knots = interp_slice(
//             &ref_knots.iter().map(|el| *el as f64).collect::<Vec<f64>>(), 
//             &query_knots.iter().map(|el| *el as f64).collect::<Vec<f64>>(),  
//             &interp_vals,
//             &InterpMode::FirstLast
//         );

//         let mut file = std::fs::File::create(format!("/home/vincent/projects/resquiggle_tool/rustmora/understanding_knots/{}_ref_to_read_knots.txt", self.read_id)).unwrap();
//         for el in &ref_to_read_knots {
//             writeln!(file, "{}", el);
//         }

//         if let Some(query_to_signal) = &self.query_to_signal {
//             let mut query_to_signal_as_f64 = Vec::new();
//             let mut query_to_signal_x_vals = Vec::new();
//             for (i, val) in query_to_signal.iter().enumerate() {
//                 query_to_signal_as_f64.push(*val as f64);
//                 query_to_signal_x_vals.push(i as f64);
//             }
            
//             let ref_to_signal = interp_slice(
//                 &query_to_signal_x_vals, 
//                 &query_to_signal_as_f64, 
//                 &ref_to_read_knots, 
//                 &InterpMode::FirstLast
//             ).iter().map(|el| *el as i64).collect::<Vec<i64>>();

//             let mut file = std::fs::File::create(format!("/home/vincent/projects/resquiggle_tool/rustmora/understanding_knots/{}_ref_to_signal.txt", self.read_id)).unwrap();
//             for el in &ref_to_signal {
//                 writeln!(file, "{}", el);
//             }
    
//             let ref_to_signal_len: i64 = ref_to_signal.len() as i64;
//             let ref_seq_len = self.bam_read.reference_end()-self.bam_read.reference_start()+1;
//             if ref_to_signal_len != ref_seq_len {
//                 return Err(AlignmentError::RefToSignalError(
//                     format!("{}: Discordant ref seq lengths. (ref_to_signal {} vs ref_seq {})", 
//                         self.pod5_read.get_id(), 
//                         ref_to_signal_len, ref_seq_len)
//                 ));
//             }

//             self.ref_to_signal = Some(ref_to_signal);
//         }

//         Ok(())
//     }

//     /// Calculate knots from a given sigar vector. Calculates either the reference knots or the query knots
//     /// based on the provided function. Use consumes_reference to calculate reference knots and 
//     /// consumes_query to calculate query knots.
//     /// 
//     /// # Arguments
//     /// * `cigar` - Vector containing the cigar elements of the alignment
//     /// * `consumes_fn` - Function that takes a reference to a Cigar element and determines if it consumes the reference
//     /// or the query (intended to be used with consumes_reference or consumes_query)
//     fn calculate_knots(&self, cigar: &Vec<Cigar>, consumes_fn: &dyn Fn(&Cigar) -> bool) -> Vec<u32> {
//         let mut total = 0;
//         let mut ref_knots = vec![0u32];

//         for el in cigar.iter() {
//             if consumes_fn(el) {
//                 total += el.len();
//             }
//             if CombinedRead::is_match_ops(el) {
//                 let offset = el.len();
//                 ref_knots.push(total-offset);
//                 ref_knots.push(total-1);
//             }
//         }
//         ref_knots.push(total);
//         ref_knots
//     }
//     // fn calculate_knots(&self, cigar: &Vec<Cigar>) -> (Vec<u32>, Vec<u32>) {
//     //     let mut current_site = 0u32;
//     //     let mut query_knots = vec![0u32];
//     //     let mut ref_knots = vec![0u32];

//     //     for el in cigar.iter() {
//     //         let cig_len = el.len();
//     //         let start = current_site;
//     //         let end = current_site + cig_len - 1;

//     //         if CombinedRead::consumes_query(el) {
//     //             query_knots.push(start);
//     //             query_knots.push(end);
//     //         }
//     //         if CombinedRead::consumes_reference(el) {
//     //             ref_knots.push(start);
//     //             ref_knots.push(end);
//     //         }
//     //         current_site += cig_len;
//     //     }    
//     //     (query_knots, ref_knots)
//     // }

//     /// Determine if the given cigar element is one of Match (M), Equal (=) or Diff (X)
//     fn is_match_ops(cigar: &Cigar) -> bool {
//         if let Cigar::Match(_) | Cigar::Equal(_) | Cigar::Diff(_) = cigar {
//             true
//         } else {
//             false
//         }
//     }

//     /// Determine if the given cigar element consumes the reference
//     /// (i.e. one of Match (M), Deletion (D), RefSkip (N), Equal (=) or Mismatch (X))
//     fn consumes_reference(cigar: &Cigar) -> bool {
//         if let Cigar::Match(_) 
//             | Cigar::Del(_) 
//             | Cigar::RefSkip(_)
//             | Cigar::Equal(_)
//             | Cigar::Diff(_) = cigar {
//             true
//         } else {
//             false
//         }
//     }

//     /// Determine if the given cigar element consumes the query
//     /// (i.e. one of Match (M), Insertion (I), SoftClip (S), Equal (=) or Mismatch (X))
//     fn consumes_query(cigar: &Cigar) -> bool {
//         if let Cigar::Match(_) 
//             | Cigar::Ins(_) 
//             | Cigar::SoftClip(_)
//             | Cigar::Equal(_)
//             | Cigar::Diff(_) = cigar {
//             true
//         } else {
//             false
//         }
//     }

//     /// Get a reference to the reference to signal alignment if it has been calculated. 
//     /// Returns a RefToSignalError otherwise.
//     pub fn get_ref_to_signal(&self) -> Result<&Vec<i64>, AlignmentError> {
//         match self.ref_to_signal {
//             Some(ref qts) => Ok(qts),
//             None => Err(AlignmentError::RefToSignalNotFound)
//         }
//     }

//     // fn extract_query_sequence(&self) -> String {
//     //     self.bam_read.seq().as_bytes().iter().map(|el| char::from(*el)).collect::<String>()
//     // }

//     // fn reverse_complement(seq: String) -> String {
//     //     seq.chars().rev().map(|base| {
//     //         match base {
//     //             'A' => 'T',
//     //             'T' => 'A',
//     //             'C' => 'G', 
//     //             'G' => 'C',
//     //             'B' => 'V',
//     //             'V' => 'B',
//     //             'D' => 'H',
//     //             'H' => 'D',
//     //             'K' => 'M',
//     //             'M' => 'K',
//     //             'R' => 'Y',
//     //             'Y' => 'R',
//     //             _ => 'X'  
//     //         }
//     //     }).collect::<String>()
//     // }
// }