use std::io::{Read, Seek, SeekFrom};

use crate::error::feather_reader::BoundedReaderError;

/// A bounded reader that constrains access to a specific region of an underlying reader.
/// This ensures that only the data between `start_offset` and `start_offset + length` 
/// can be accessed, providing a virtual "file" view of embedded data.
#[derive(Debug)]
pub(crate) struct BoundedReader<R: Read + Seek> {
    inner: R,
    start_offset: u64,
    length: u64,
    current_pos: u64
}

impl<R: Read + Seek> BoundedReader<R> {
    pub(crate) fn new(mut reader: R, offset: u64, length: u64) -> Result<Self, BoundedReaderError> {
        reader.seek(std::io::SeekFrom::Start(offset))?;

        Ok(BoundedReader { 
            inner: reader, 
            start_offset: offset, 
            length, 
            current_pos: 0 
        })
    }

    pub(crate) fn reset(&mut self) -> std::io::Result<()> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }
}

impl<R: Read + Seek> Read for BoundedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self.length.saturating_sub(self.current_pos);
        if remaining == 0 {
            return Ok(0);
        }

        let to_read = std::cmp::min(buf.len() as u64, remaining) as usize;
        let bytes_read = self.inner.read(&mut buf[..to_read])?;
        self.current_pos += bytes_read as u64;

        Ok(bytes_read)
    }
}

impl<R: Read + Seek> Seek for BoundedReader<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if offset > 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput, 
                        "Cannot seek past end of embedded data"
                    ));
                }
                (self.length as i64 + offset) as u64
            },
            SeekFrom::Current(offset) => {
                if offset < 0 && (-offset) as u64 > self.current_pos {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Cannot seek before start of embedded data",
                    ));
                }
                (self.current_pos as i64 + offset) as u64
            }        
        };

        if new_pos > self.length {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot seek past end of embedded data",
            ));
        }

        self.inner.seek(SeekFrom::Start(self.start_offset + new_pos))?;
        self.current_pos = new_pos;

        Ok(new_pos)
    }
}