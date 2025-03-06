use bitvec::vec;
use rust_htslib::bam::record::Cigar;

use crate::error::loader_errors::bam_errors::RefSeqReconstructError;

const A: u8 = 65;
const C: u8 = 67;
const G: u8 = 71;
const T: u8 = 84;

const A_low: u8 = 97;
const C_low: u8 = 99;
const G_low: u8 = 103;
const T_low: u8 = 116;

#[derive(Debug, PartialEq)]
pub enum MDOperation {
    Match(usize),
    Mismatch(u8),
    Deletion(Vec<u8>),
    Insertion,
    Ignore
}

pub struct RefSeqReconstructor<'a> {
    query: &'a Vec<u8>,
    md_operations: Vec<MDOperation>,
    cigar: &'a Vec<Cigar>
}

impl<'a> RefSeqReconstructor<'a> {
    pub fn new(query: &'a Vec<u8>, md_vec: &[u8], cigar: &'a Vec<Cigar>) -> Self {
        let md_operations = parse_md_string(md_vec);

        RefSeqReconstructor {
            query,
            md_operations,
            cigar
        }
    }

    pub fn get_reference_sequence(&self) -> Vec<u8> {
        todo!()
    }
}

fn parse_md_string(md_vec: &[u8]) -> Result<Vec<MDOperation>, RefSeqReconstructError> {
    let mut md_operations = Vec::new();
    let mut idx = 0;
    while idx < md_vec.len() {
        let (idx_increase, operation) = determine_current_operation(&md_vec[idx..])?;
        idx += idx_increase;
        md_operations.push(operation);
    }

    md_operations
}

fn determine_current_operation(md_slice: &[u8]) -> Result<(usize, MDOperation), RefSeqReconstructError> {
    let start_char = md_slice[0];
    if md_slice.len() < 1 {
        Err(RefSeqReconstructError::EmptyMdSlice)
    }
    else if is_numeric_nonzero(start_char) {
        // Extract all digits until a non-digit char comes
        // these digits make up the number of matches
        let mut digits_u8_slice = vec![start_char];
        let mut idx = 1;
        // Collect characters until a non-digit one is found
        while is_numeric(md_slice[idx]) && idx < md_slice.len() {
            digits_u8_slice.push(md_slice[idx]);
            idx += 1
        }
        // Convert collected u8 values to usize
        let digits_str = std::str::from_utf8(&digits_u8_slice)?;
        let num = digits_str.parse::<usize>()?;
        Ok((idx, MDOperation::Match(num)))
    }
    else if is_zero(start_char) {
        // Ignore (just a buffer to split up non-connected base sequences)
        Ok((1, MDOperation::Ignore))
    }
    else if is_base(start_char) {
        Ok((1, MDOperation::Mismatch(start_char)))
    }
    else if is_circumflex(start_char) {
        let mut deleted_bases = Vec::new();
        let mut idx = 1;
        // Collect characters until a digit is found
        while !is_numeric(md_slice[idx]) && idx < md_slice.len() {
            deleted_bases.push(md_slice[idx]);
            idx += 1;
        }
        Ok((idx, MDOperation::Deletion(deleted_bases)))
    } 
    else {
        Err(RefSeqReconstructError::UnexpectedChar(start_char))
    }
} 

/// Checks if the encoded character is a digit (0 to 9)
fn is_numeric(c: u8) -> bool { c >= 48 && c <= 57 }

/// Checks if the encoded character is a digit larger than 0
fn is_numeric_nonzero(c: u8) -> bool { c > 48 && c <= 57 }

/// Checks if the encoded character is '0'
fn is_zero(c: u8) -> bool { c == 48 }

/// Checks if the encoded character is one of 'A'/'C'/'G'/'T'
/// or lowercase versions
fn is_base(c: u8) -> bool {
    c == A || c == C || c == G || c == T 
    || c == A_low || c == C_low || c == G_low || c == T_low
}

/// Checks if the encoded character is '^'
fn is_circumflex(c: u8) -> bool { c == 94 }



#[cfg(test)]
mod test {
    use std::result;

    use super::*;

    #[test]
    fn test_determine_current_operation() {
        let md_string = "3A140CGT^AAAAA0TG50".to_string();
        let md_vec = md_string.as_bytes();

        let result = determine_current_operation(&md_vec[0..]);
        let expected = Ok((1, MDOperation::Match(3)));
        assert_eq!(result, expected);
        
        let result = determine_current_operation(&md_vec[1..]);
        let expected = Ok((1, MDOperation::Mismatch(A)));
        assert_eq!(result, expected);

        let result = determine_current_operation(&md_vec[2..]);
        let expected = Ok((3, MDOperation::Match(140)));
        assert_eq!(result, expected);

        let result = determine_current_operation(&md_vec[5..]);
        let expected = Ok((1, MDOperation::Mismatch(C)));
        assert_eq!(result, expected);

        let result = determine_current_operation(&md_vec[8..]);
        let expected = Ok((6, MDOperation::Deletion(vec![A, A, A, A, A])));
        assert_eq!(result, expected);

        let md_string = "".to_string();
        let md_vec = md_string.as_bytes();
        let result = determine_current_operation(&md_vec[0..]);
        let expected = Err(RefSeqReconstructError::EmptyMdSlice);
        assert_eq!(result, expected);

        let md_string = "3X140CGT^AAAAA0TG50".to_string();
        let md_vec = md_string.as_bytes();
        let result = determine_current_operation(&md_vec[1..]);
        let expected = Err(RefSeqReconstructError::UnexpectedChar(88));
        assert_eq!(result, expected);
    }
}