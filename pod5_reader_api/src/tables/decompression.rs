//! THIS IS DIRECTLY TAKEN FROM THE pod5-rs CRATE!
//! I ONLY REMOVED UNUSED ENCODING CODE!
//! 
//! https://github.com/bsaintjo/pod5-rs/blob/main/svb16/src/lib.rs
//! 
//! This implements the compression algorithm used in POD5 format.
//!
//! POD5 uses a variant of the streamvbyte algorithm. Since signal values are
//! only 16-bit (i16) values, it only needs to consider if values fit into 1
//! data byte or 2 data bytes. This means that it only needs to use 1-bit to
//! encode the size, so every control byte encodes up to 8 values, instead of
//! 4..

use std::io;

use bitvec::{prelude::Lsb0, slice::Iter, view::BitView};
use delta_encoding::DeltaDecoderExt;
use zigzag::ZigZag;

// TODO could remove idx, and just mutate the data field in place
struct DecodeIter<'a> {
    count: usize,
    samples: usize,
    bits: Iter<'a, u8, Lsb0>,
    idx: usize,
    data: &'a [u8],
}

impl<'a> DecodeIter<'a> {
    fn new(ctrl_bytes: &'a [u8], data: &'a [u8], samples: usize) -> Self {
        Self {
            bits: ctrl_bytes.view_bits().iter(),
            idx: 0,
            data,
            count: 0,
            samples,
        }
    }

    fn from_compressed(data: &'a [u8], samples: usize) -> Self {
        let (ctrl, data) = split_data(data, samples);
        DecodeIter::new(ctrl, data, samples)
    }
}

impl Iterator for DecodeIter<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == self.samples {
            return None;
        }
        let code = self.bits.next()?;
        let value = if *code {
            // Bit is set to 1, so two-bytes need to be parsed
            let tmp = u16::from_le_bytes(self.data[self.idx..self.idx + 2].try_into().unwrap());
            self.idx += 2;
            tmp
        } else {
            // Bit is set to 0, so only one byte is needed
            let tmp = self.data[self.idx] as u16;
            self.idx += 1;
            tmp
        };
        self.count += 1;
        Some(value)
    }
}

/// zstd -> streamvbyte -> zig-zag -> delta
/// Can panic if the compressed array doesn't follow the SVB16 specification.
///
/// When running on compressed signal data from a signal column in a POD5 file,
/// use `decode` on the individual rows. If you try to combine the compressed
/// signal across multiple rows that correspond to a signal read this function
/// will panic.
pub fn decode(compressed: &[u8], count: usize) -> io::Result<Vec<i16>> {
    let compressed = zstd::decode_all(compressed)?;
    Ok(DecodeIter::from_compressed(&compressed, count)
        .map(ZigZag::decode)
        .original()
        .collect())
}

fn split_data(compressed: &[u8], count: usize) -> (&[u8], &[u8]) {
    let mid = num_ctrl_bytes(count);
    compressed.split_at(mid)
}

/// Get number of control bytes used in this variant of streamvbyte
///
/// Essential ceil(count / 8) but we copy the bit operator version from
/// nanopore/pod5-file-format
fn num_ctrl_bytes(count: usize) -> usize {
    // (count as f64 / 8.).ceil() as usize
    (count >> 3) + (((count & 7) + 7) >> 3)
}

// fn max_encoded_length(count: usize) -> usize {
//     num_ctrl_bytes(count) + (2 * count)
// }
