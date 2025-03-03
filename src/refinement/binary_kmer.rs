use super::super::error::refinement_errors::kmer_table_errors::BinaryKmerError;

/// Represents a k-mer using a compact binary encoding
///
/// Each nucleotide is encoded using 2 bits:
/// - A: 00 (0)
/// - C: 01 (1)
/// - G: 10 (2)
/// - T: 11 (3)
///
/// For k-mers with k ≤ 32, a single u64 can store the entire k-mer.
/// For longer k-mers, a Vec<u64> would be required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BinaryKmer {
    encoded: u64,
    k: usize
}

impl BinaryKmer {
    /// Maximum k-mer length that can be stored in a single u64
    /// 64 bits / 2 bits per nucleotide = 32 nucleotides
    pub const MAX_K: usize = 32;

    /// Converts a string k-mer to its binary representation
    ///
    /// Works with both T and U nucleotides, handling U as T internally.
    /// All input characters are converted to uppercase before processing.
    ///
    /// # Arguments
    ///
    /// * `kmer` - A string slice containing only A, C, G, T/U characters
    ///
    /// # Returns
    ///
    /// * `Result<BinaryKmer, BinaryKmerError>` - Binary encoding or an error
    ///
    /// # Errors
    ///
    /// * `BinaryKmerError::InvalidKmerLen` - If the k-mer is too long to fit in the binary representation
    /// * `BinaryKmerError::InvalidBaseChar` - If the k-mer contains invalid characters (not A, C, G, T, or U)
    pub fn from_string(kmer: &str) -> Result<Self, BinaryKmerError> {
        let kmer = kmer.to_uppercase();
        let k = kmer.len();
        if k > Self::MAX_K {
            return Err(BinaryKmerError::InvalidKmerLen(k));
        }

        let mut encoded: u64 = 0;

        for nucleotide in kmer.chars() {
            let base_code = match nucleotide {
                'A' => 0u64,
                'C' => 1u64,
                'G' => 2u64,
                'T' => 3u64,
                'U' => 3u64,
                _ => return Err(BinaryKmerError::InvalidBaseChar(nucleotide))
            };
            // Shift and incorporate the new nucleotide
            encoded = (encoded << 2) | base_code;
        }
        
        Ok(BinaryKmer{ encoded, k })
    }

    /// Converts the binary k-mer back to a string representation
    /// 
    /// # Returns
    ///
    /// * `String` - The string representation of the k-mer using A, C, G, T characters
    pub fn to_string(&self) -> String {
        let mut result = String::with_capacity(self.k);
        let mut mask = self.encoded;

        for _ in 0..self.k {
            let code = mask & 0b11;
            let nucleotide = match code {
                0 => 'A',
                1 => 'C',
                2 => 'G',
                3 => 'T',
                _ => unreachable!(), // This should never happen (masking with 0b11)
            };
            result.insert(0, nucleotide);
            mask >>= 2;
        }
        result
    }

    /// Extracts the nucleotide at a specific position in the k-mer
    ///
    /// # Arguments
    ///
    /// * `position` - The position (0-based, from the left) to extract
    ///
    /// # Returns
    ///
    /// * `Result<char, BinaryKmerError>` - The nucleotide ('A', 'C', 'G', or 'T') or an error
    ///
    /// # Errors
    ///
    /// * `BinaryKmerError::PositionIndexError` - If the position is out of bounds
    pub fn nucleotide_at(&self, position: usize) -> Result<char, BinaryKmerError> {
        if position >= self.k {
            return Err(BinaryKmerError::PositionIndexError(position, self.k));
        }
        // Calculate bit position (from the right)
        let bit_pos = (self.k - position - 1) * 2;
        // Extract the 2 bits at that position
        let code = (self.encoded >> bit_pos) & 0b11;

        let nuc = match code {
            0 => 'A',
            1 => 'C',
            2 => 'G',
            3 => 'T',
            _ => unreachable!(), // This should never happen
        };
        Ok(nuc)
    }

    /// Returns the k (length) for the current k-mer
    ///
    /// # Returns
    ///
    /// * `usize` - The length of the k-mer
    pub fn k(&self) -> usize {
        self.k
    }
}










#[cfg(test)]
mod binary_kmer_tests {
    use super::*;

    #[test]
    fn test_binary_kmer_creation() {
        // Valid k-mers
        let kmer1 = BinaryKmer::from_string("ACGT");
        assert!(kmer1.is_ok());
        let kmer1 = kmer1.unwrap();
        assert_eq!(kmer1.k(), 4);
        assert_eq!(kmer1.to_string(), "ACGT");

        // Test case insensitivity
        let kmer2 = BinaryKmer::from_string("acgt");
        assert!(kmer2.is_ok());
        assert_eq!(kmer2.unwrap().to_string(), "ACGT");

        // Test with U (should be treated as T)
        let kmer3 = BinaryKmer::from_string("ACGU");
        assert!(kmer3.is_ok());
        assert_eq!(kmer3.unwrap().to_string(), "ACGT");

        // Invalid characters
        let invalid_kmer = BinaryKmer::from_string("ACGN");
        assert!(invalid_kmer.is_err());
        match invalid_kmer {
            Err(BinaryKmerError::InvalidBaseChar(c)) => assert_eq!(c, 'N'),
            _ => panic!("Expected InvalidBaseChar error"),
        }

        // Test max length
        let max_valid = "A".repeat(BinaryKmer::MAX_K);
        assert!(BinaryKmer::from_string(&max_valid).is_ok());

        // Test too long
        let too_long = "A".repeat(BinaryKmer::MAX_K + 1);
        let too_long_result = BinaryKmer::from_string(&too_long);
        assert!(too_long_result.is_err());
        match too_long_result {
            Err(BinaryKmerError::InvalidKmerLen(len)) => assert_eq!(len, BinaryKmer::MAX_K + 1),
            _ => panic!("Expected InvalidKmerLen error"),
        }
    }

    #[test]
    fn test_nucleotide_at() {
        let kmer = BinaryKmer::from_string("ACGT").unwrap();
        
        assert_eq!(kmer.nucleotide_at(0).unwrap(), 'A');
        assert_eq!(kmer.nucleotide_at(1).unwrap(), 'C');
        assert_eq!(kmer.nucleotide_at(2).unwrap(), 'G');
        assert_eq!(kmer.nucleotide_at(3).unwrap(), 'T');
        
        // Test out of bounds
        let out_of_bounds = kmer.nucleotide_at(4);
        assert!(out_of_bounds.is_err());
        match out_of_bounds {
            Err(BinaryKmerError::PositionIndexError(pos, k)) => {
                assert_eq!(pos, 4);
                assert_eq!(k, 4);
            },
            _ => panic!("Expected PositionIndexError"),
        }
    }

    #[test]
    fn test_binary_encoding() {
        // Test that each nucleotide is encoded correctly
        // A: 00, C: 01, G: 10, T: 11
        
        // Test "A" (00) => 0
        let kmer_a = BinaryKmer::from_string("A").unwrap();
        assert_eq!(kmer_a.encoded, 0);
        
        // Test "C" (01) => 1
        let kmer_c = BinaryKmer::from_string("C").unwrap();
        assert_eq!(kmer_c.encoded, 1);
        
        // Test "G" (10) => 2
        let kmer_g = BinaryKmer::from_string("G").unwrap();
        assert_eq!(kmer_g.encoded, 2);
        
        // Test "T" (11) => 3
        let kmer_t = BinaryKmer::from_string("T").unwrap();
        assert_eq!(kmer_t.encoded, 3);
        
        // Test "ACGT" (00 01 10 11) => binary 00011011 => decimal 27
        let kmer_acgt = BinaryKmer::from_string("ACGT").unwrap();
        assert_eq!(kmer_acgt.encoded, 27);
        
        // Test "TGCA" (11 10 01 00) => binary 11100100 => decimal 228
        let kmer_tgca = BinaryKmer::from_string("TGCA").unwrap();
        assert_eq!(kmer_tgca.encoded, 228);
    }

    #[test]
    fn test_binary_kmer_equality() {
        let kmer1 = BinaryKmer::from_string("ACGT").unwrap();
        let kmer2 = BinaryKmer::from_string("ACGT").unwrap();
        let kmer3 = BinaryKmer::from_string("TGCA").unwrap();
        
        assert_eq!(kmer1, kmer2);
        assert_ne!(kmer1, kmer3);
        
        // Different k-mers with same encoded value but different lengths
        let kmer4 = BinaryKmer::from_string("A").unwrap();  // encoded: 0, k: 1
        let kmer5 = BinaryKmer::from_string("AA").unwrap(); // encoded: 0, k: 2
        assert_ne!(kmer4, kmer5);
    }
}