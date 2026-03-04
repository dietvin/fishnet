/*!
 * This build script parses kmer tables into KmerTableData objects
 * and serializes them into binary format for embedding during
 * compilation.
 */

use std::{fs::File, io::Write, path::PathBuf};
use kmer_table::kmer_table_data::KmerTableData;


/// Serialize a kmer table into binary format
/// 
/// Reads a kmer table text file, parses it into a KmerTableData instance,
/// encodes it into bytes and writes these to a file.
fn serialize_kmer_table(
    kmer_table_file_name: &str,
    bin_file_name: &str,
    is_legacy: bool
) {
    let kmer_table_path = PathBuf::from(kmer_table_file_name);
    
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let bin_path = PathBuf::from(out_dir).join(bin_file_name);

    let kmer_table_data: KmerTableData = KmerTableData::from_file(
        &kmer_table_path,
        is_legacy,
        is_legacy
    ).unwrap();
    let encoded = bincode::serialize(&kmer_table_data).unwrap();
    let mut file = File::create(bin_path).unwrap();
    file.write_all(&encoded).unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=kmer_models");

    serialize_kmer_table(
        "kmer_models/dna_r10.4.1_e8.2_260bps/9mer_levels_v1.txt",
        "kmer_table_data_dna_r10_260bps.bin",
        false
    );

    serialize_kmer_table(
        "kmer_models/dna_r10.4.1_e8.2_400bps/9mer_levels_v1.txt",
        "kmer_table_data_dna_r10_400bps.bin",
        false
    );

    serialize_kmer_table(
        "kmer_models/rna_r9.4_180mv_70bps/5mer_levels_v1.txt",
        "kmer_table_data_rna002.bin",
        false
    );

    serialize_kmer_table(
        "kmer_models/rna004/9mer_levels_v1.txt",
        "kmer_table_data_rna004.bin",
        false
    );

    serialize_kmer_table(
        "kmer_models/legacy/legacy_r9.4_180mv_450bps_6mer/template_median68pA.model",
        "kmer_table_data_dna_r9_450bps.bin",
        true
    );
}
