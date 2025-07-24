use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use pod5_reader_api::file::Pod5File;



type SignalData = HashMap<String, Vec<i16>>;

fn load_data() -> SignalData {
    let file = File::open("tests/signals.json").unwrap();
    let reader = BufReader::new(file);

    let data: SignalData = serde_json::from_reader(reader).unwrap();
    data
}

#[test]
fn test_signal_resconstruction() {
    let path = PathBuf::from("../example_data/can_reads.pod5");
    let mut pod5_file = Pod5File::new(path).unwrap();
    let read_iter = pod5_file.iter_reads().unwrap();

    let expected_signals = load_data();

    for read_res in read_iter {
        let read = read_res.unwrap();
        let read_id = read.read_id().to_string();
        let signal = read.signal().unwrap();

        let expected_signal = expected_signals.get(&read_id).unwrap();
        assert_eq!(signal, expected_signal)
    } 
}