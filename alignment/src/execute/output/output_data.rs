use crate::{
    core::refinement::signal_map_refiner::SigMapRefiner, execute::config::{output_config::OutputConfig, WhichToAlign}
};

/// Represents different schemas to write to file
#[derive(Debug, Clone)]
pub enum OutputData {

    /// Minimal output format containing only the alignments (Ruery alignment only)
    QueryBasic {
        read_id: String,
        query_to_signal: Option<Vec<usize>>
    },

    /// Minimal output format containing only the alignments (reference alignment only)
    RefBasic {
        read_id: String,
        ref_to_signal: Option<Vec<usize>>,
        ref_name: Option<String>,
        ref_start: Option<usize>
    },

    /// Minimal output format containing only the alignments (Query + reference alignments)
    BothBasic {
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
        ref_name: Option<String>,
        ref_start: Option<usize>
    },


    /// Extended output format containing the sequences in addition to the alignments
    /// (Ruery alignment only)
    QueryWithSeq {
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>
    },

    /// Extended output format containing the sequences in addition to the alignments
    /// (Reference alignment only)
    RefWithSeq {
        read_id: String,
        ref_to_signal: Option<Vec<usize>>,
        ref_sequence: Option<String>,
        ref_name: Option<String>,
        ref_start: Option<usize>
    },

    /// Extended output format containing the sequences in addition to the alignments
    /// (Query + reference alignments)
    BothWithSeq {
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>,
        ref_to_signal: Option<Vec<usize>>,
        ref_sequence: Option<String>,
        ref_name: Option<String>,
        ref_start: Option<usize>
    },


    /// Extended output format containing the alignements, the sequences and the signal
    /// (Ruery alignment only)
    QueryWithSeqAndSig {
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>,
        signal: Option<Vec<i16>>
    },

    /// Extended output format containing the alignements, the sequences and the signal
    /// (Reference alignment only)
    RefWithSeqAndSig {
        read_id: String,
        ref_to_signal: Option<Vec<usize>>,
        ref_sequence: Option<String>,
        ref_name: Option<String>,
        ref_start: Option<usize>,
        signal: Option<Vec<i16>>
    },

    /// Extended output format containing the alignements, the sequences and the signal
    /// (Query + reference alignments)
    BothWithSeqAndSig {
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>,
        ref_to_signal: Option<Vec<usize>>,
        ref_sequence: Option<String>,
        ref_name: Option<String>,
        ref_start: Option<usize>,
        signal: Option<Vec<i16>>
    }
}

/// Constructor functions for each option
impl OutputData {
    pub fn new(
        output_config: &OutputConfig,
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
        sig_map_refiner: &SigMapRefiner,
        is_rna: bool
    ) -> Self {
        let alignment_type = output_config.alignment_type();

        let ref_name = match alignment_type {
            WhichToAlign::Query => None,
            WhichToAlign::Reference | WhichToAlign::Both => sig_map_refiner
                .bam_read()
                .get_reference_name()
                .cloned()
                .ok()
        };

        let ref_start = match alignment_type {
            WhichToAlign::Query => None,
            WhichToAlign::Reference | WhichToAlign::Both => sig_map_refiner
                .bam_read()
                .get_reference_start()
                .cloned()
                .ok()
        };

        // Level 1: Only the alignment(s)
        if !output_config.include_sequences() {
            return OutputData::basic(
                read_id, 
                query_to_signal, 
                ref_to_signal, 
                ref_name, 
                ref_start, 
                alignment_type
            );
        }
        // Level 2: Alignment(s) + sequence(s)
        else {
            // Get the query sequence frim the sig map refiner if needed
            let query_sequence = match alignment_type {
                WhichToAlign::Query | WhichToAlign::Both => query_to_signal
                    .as_ref()
                    .and_then(|_| {
                        String::from_utf8(sig_map_refiner
                            .bam_read()
                            .query()
                            .to_vec()
                        ).ok()
                    }),
                WhichToAlign::Reference => None
            };

            // Get the reference sequence frim the sig map refiner if needed
            let ref_sequence = match alignment_type {
                WhichToAlign::Query => None,
                WhichToAlign::Reference | WhichToAlign::Both => ref_to_signal
                    .as_ref()
                    .and_then(|_| {
                        sig_map_refiner
                            .bam_read()
                            .get_reference()
                            .ok()
                            .and_then(|s_u8| String::from_utf8(s_u8.clone()).ok())
                    })
                };
            
            if !output_config.include_signal() {
                return OutputData::with_seq(
                    read_id,
                    query_to_signal,
                    ref_to_signal,
                    query_sequence,
                    ref_sequence,
                    ref_name,
                    ref_start,
                    alignment_type
                );
            } 
            // Level 3: Alignment(s) + sequence(s) + signal
            else {
                let signal = match ref_to_signal.is_some() || query_to_signal.is_some() {
                    true => {
                        let signal = sig_map_refiner.pod5_read().signal().cloned();
                        if let Some(mut sig) = signal {
                            // Reverse the signal to match the alignment directly in the output
                            if is_rna {
                                sig.reverse();
                            }
                            Some(sig)
                        } else {
                            None
                        }
                    }
                    false => None
                };
                return OutputData::with_seq_and_signal(
                        read_id, 
                        query_to_signal, 
                        ref_to_signal, 
                        query_sequence,
                        ref_sequence,
                        ref_name,
                        ref_start,
                        signal,
                        alignment_type
                    )
            }
        }
    }

    /// Initializes with basic output data
    fn basic(
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
        ref_name: Option<String>,
        ref_start: Option<usize>,
        alignment_type: &WhichToAlign
    ) -> Self {
        match alignment_type {
            WhichToAlign::Query => OutputData::QueryBasic { 
                read_id, 
                query_to_signal
            },
            WhichToAlign::Reference => OutputData::RefBasic { 
                read_id, 
                ref_to_signal, 
                ref_name,
                ref_start 
            },
            WhichToAlign::Both => OutputData::BothBasic { 
                read_id, 
                query_to_signal, 
                ref_to_signal, 
                ref_name,
                ref_start
            }
        }
    }

    /// Initializes with sequence(s) included
    fn with_seq(
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>,
        ref_sequence: Option<String>,
        ref_name: Option<String>,
        ref_start: Option<usize>,
        alignment_type: &WhichToAlign
    ) -> Self {
        match alignment_type {
            WhichToAlign::Query => OutputData::QueryWithSeq { 
                read_id, 
                query_to_signal, 
                query_sequence
            },
            WhichToAlign::Reference => OutputData::RefWithSeq { 
                read_id,
                ref_to_signal,
                ref_sequence,
                ref_name,
                ref_start
            },
            WhichToAlign::Both => OutputData::BothWithSeq { 
                read_id,
                query_to_signal,
                query_sequence,
                ref_to_signal,
                ref_sequence,
                ref_name,
                ref_start
            }
        }

    }

    /// Initializes with both sequence(s) and signal included
    fn with_seq_and_signal(
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>,
        ref_sequence: Option<String>,
        ref_name: Option<String>,
        ref_start: Option<usize>,
        signal: Option<Vec<i16>>,
        alignment_type: &WhichToAlign
    ) -> Self {
        match alignment_type {
            WhichToAlign::Query => OutputData::QueryWithSeqAndSig { 
                read_id,
                query_to_signal,
                query_sequence,
                signal
            },
            WhichToAlign::Reference => OutputData::RefWithSeqAndSig { 
                read_id,
                ref_to_signal,
                ref_sequence,
                ref_name,
                ref_start,
                signal
            },
            WhichToAlign::Both => OutputData::BothWithSeqAndSig { 
                read_id, 
                query_to_signal, 
                query_sequence, 
                ref_to_signal, 
                ref_sequence, 
                ref_name, 
                ref_start, 
                signal 
            }            
        }
    }

    /// Checks if a given output schema corresponds to the output data at hand.
    pub fn matches(&self, output_config: &OutputConfig) -> bool {
        match (
            self, 
            output_config.alignment_type(), 
            output_config.include_sequences(), 
            output_config.include_signal
        ) {
            (OutputData::QueryBasic { .. }, WhichToAlign::Query, false, false) => true,
            (OutputData::RefBasic { .. }, WhichToAlign::Reference, false, false) => true,
            (OutputData::BothBasic { .. }, WhichToAlign::Both, false, false) => true,
            (OutputData::QueryWithSeq { .. }, WhichToAlign::Query, true, false) => true,
            (OutputData::RefWithSeq { .. }, WhichToAlign::Reference, true, false) => true,
            (OutputData::BothWithSeq { .. }, WhichToAlign::Both, true, false) => true,
            (OutputData::QueryWithSeqAndSig { .. }, WhichToAlign::Query, true, true) => true,
            (OutputData::RefWithSeqAndSig { .. }, WhichToAlign::Reference, true, true) => true,
            (OutputData::BothWithSeqAndSig { .. }, WhichToAlign::Both, true, true) => true,
            _ => false
        }
    }

    /// Returns the name of the variant
    pub fn variant_name(&self) -> &'static str {
        match self {
            OutputData::QueryBasic { .. } => "QueryBasic",
            OutputData::RefBasic { .. } => "RefBasic",
            OutputData::BothBasic { .. } => "BothBasic",
            OutputData::QueryWithSeq { .. } => "QueryWithSeq",
            OutputData::RefWithSeq { .. } => "RefWithSeq",
            OutputData::BothWithSeq { .. } => "BothWithSeq",
            OutputData::QueryWithSeqAndSig { .. } => "QueryWithSeqAndSig",
            OutputData::RefWithSeqAndSig { .. } => "RefWithSeqAndSig",
            OutputData::BothWithSeqAndSig { .. } => "BothWithSeqAndSig",
        }
    }
}
