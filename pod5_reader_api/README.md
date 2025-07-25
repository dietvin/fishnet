# POD5 Reader API

A light-weight and minimal Rust library for reading and processing POD5 files. This library provides efficient access to sequencing reads, run information, and signal data contained within POD5 files.

## Overview

The POD5 Reader API consists of three main modules that work together to provide access to POD5 file contents:

- **Pod5Dataset**: Manages collections of multiple POD5 files
- **Pod5File**: Handles individual POD5 file operations and data access
- **Pod5Read**: Represents individual sequencing reads with metadata and signal data

## Core Modules

### Pod5Dataset

The `Pod5Dataset` struct provides a unified interface for working with multiple POD5 files as a single dataset. It offers both indexed and path-based access to individual files.

**Key Features:**
- **Multi-file Management**: Handle collections of POD5 files seamlessly
- **Dual Access Methods**: Access files by path (using `OsString` keys) or by numerical index
- **Memory Efficient**: Lazy loading and efficient indexing
- **Iterator Support**: Built-in iteration over all files in the dataset

**Usage Example:**
```rust
// Create a dataset from multiple POD5 files
let paths = vec![
    PathBuf::from("file1.pod5"),
    PathBuf::from("file2.pod5"),
];
let dataset = Pod5Dataset::new(&paths)?;

// Access files by path or index
let file_by_path = dataset.get(&OsString::from("file1.pod5"))?;
let file_by_index = dataset.get_by_index(0)?;

// Iterate over all files
for file in dataset.iter_files() {
    println!("File: {:?}, Reads: {}", file.path(), file.n_reads());
}
```

### Pod5File

The `Pod5File` struct represents a single POD5 file and handles all low-level file operations. It implements a multi-stage parsing approach for optimal performance.

**Parsing Pipeline:**
1. **Signature Verification**: Validates POD5 file format signatures
2. **Footer Parsing**: Extracts file structure metadata
3. **Run Info Extraction**: Loads sequencing run information
4. **Reads Table Parsing**: Builds index of all reads in the file
5. **Lazy Signal Loading**: Loads signal data on-demand

**Key Features:**
- **Efficient Memory Usage**: Signal data loaded only when requested
- **Fast Read Access**: HashMap-based read lookup by UUID
- **Iterator Support**: Stream through all reads without loading everything into memory
- **Comprehensive Error Handling**: Detailed error reporting for file corruption or access issues

**Usage Example:**
```rust
// Open a POD5 file
let mut file = Pod5File::new(PathBuf::from("data.pod5"))?;

// Access run information
let run_info = file.run_info();
println!("Device ID: {}", run_info.device_id);

// Get a specific read (loads signal data)
let read_id = file.read_ids()[0];
let read = file.get(&read_id)?;

// Iterate through all reads
for read_result in file.iter_reads()? {
    let read = read_result?;
    println!("Read ID: {}, Samples: {}", read.read_id(), read.num_samples().unwrap_or(0));
}
```

### Pod5Read

The `Pod5Read` struct represents an individual sequencing read with all associated metadata and signal data. It implements lazy loading to minimize memory usage.

**Metadata Fields:**
- **Identifiers**: Read ID (UUID), read number, run info ID
- **Timing**: Start time, time since mux change
- **Signal Properties**: Number of samples, signal indices for reconstruction
- **Calibration**: Offset and scale parameters for signal conversion
- **Hardware**: Pore information, channel details
- **Processing**: Scaling parameters from MinKNOW and Guppy
- **Quality**: End reason, median before value

**Key Features:**
- **Lazy Signal Loading**: Signal data loaded on-demand to save memory
- **Comprehensive Metadata**: Full access to all read-level information
- **Type Safety**: Required vs optional field access with clear error handling
- **Flexible Access**: Both optional and required field accessors

**Usage Example:**
```rust
// Access read metadata
println!("Read ID: {}", read.read_id());
println!("Read Number: {:?}", read.read_number());
println!("Channel: {}", read.pore().channel);

// Access required fields (with error handling)
let samples = read.require_num_samples()?;
let start_time = read.require_start()?;

// Access signal data (if loaded)
if let Some(signal) = read.signal() {
    println!("Signal length: {}", signal.len());
    println!("First few samples: {:?}", &signal[..5]);
}

// Access calibration parameters
let offset = read.require_calibration_offset()?;
let scale = read.require_calibration_scale()?;
```

## Error Handling

The API provides comprehensive error handling through custom error types:

- **Pod5DatasetError**: Dataset-level errors (invalid keys, index out of bounds)
- **Pod5FileError**: File-level errors (IO issues, invalid signatures, parsing failures)
- **Pod5ReadError**: Read-level errors (missing required fields)

All error types implement the `thiserror::Error` trait for excellent error messages and debugging support.

## Memory Management

The library is designed for efficient memory usage:

- **Lazy Loading**: Signal data is only loaded when explicitly requested
- **Streaming Access**: Iterator-based access prevents loading entire files into memory
- **Selective Loading**: Load only the reads and data you need
- **Efficient Indexing**: Fast UUID-based read lookup without scanning

## Dependencies

- **arrow2**: Apache Arrow implementation for columnar data access
- **uuid**: UUID handling for read identifiers
- **thiserror**: Enhanced error handling
- **std collections**: HashMap and Vec for efficient data structures

## Performance Characteristics

- **File Opening**: Fast - only metadata is parsed initially
- **Read Access**: O(1) lookup by UUID via HashMap
- **Signal Loading**: On-demand - only loaded when `get()` is called
- **Memory Usage**: Minimal until signal data is accessed
- **Iterator Performance**: Streaming - constant memory usage regardless of file size