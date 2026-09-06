//! Zero-copy reader for efficient field extraction.
//!
//! The [`ZeroCopyReader`] provides a cursor-based API for reading fields
//! from a byte buffer without deserializing the entire structure.

use thiserror::Error;

/// Errors from zero-copy reading operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ZeroCopyError {
    /// Not enough bytes remaining in the buffer.
    #[error("unexpected end of buffer: need {needed} bytes, have {remaining}")]
    UnexpectedEof { needed: usize, remaining: usize },

    /// String data is not valid UTF-8.
    #[error("invalid UTF-8 in string field")]
    InvalidUtf8,

    /// A length prefix exceeds reasonable bounds.
    #[error("length prefix too large: {length} bytes")]
    LengthTooLarge { length: usize },
}

/// A zero-copy reader for extracting fields from a byte buffer.
///
/// This reader maintains a cursor position and provides methods for reading
/// various primitive types without allocating memory for the data itself.
///
/// # Example
///
/// ```rust
/// use account_decoder_borsh_util::ZeroCopyReader;
///
/// let data = [
///     0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // u64: 1
///     0xFF,                                             // u8: 255
///     0x10, 0x00, 0x00, 0x00,                           // u32: 16
/// ];
///
/// let mut reader = ZeroCopyReader::new(&data);
///
/// assert_eq!(reader.read_u64().unwrap(), 1);
/// assert_eq!(reader.read_u8().unwrap(), 255);
/// assert_eq!(reader.read_u32().unwrap(), 16);
/// assert!(reader.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct ZeroCopyReader<'a> {
    /// The underlying byte buffer.
    data: &'a [u8],
    /// Current position in the buffer.
    pos: usize,
}

impl<'a> ZeroCopyReader<'a> {
    /// Create a new reader over a byte slice.
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Create a reader starting at an offset.
    pub fn with_offset(data: &'a [u8], offset: usize) -> Result<Self, ZeroCopyError> {
        if offset > data.len() {
            return Err(ZeroCopyError::UnexpectedEof {
                needed: offset,
                remaining: data.len(),
            });
        }
        Ok(Self { data, pos: offset })
    }

    /// Get the current position in the buffer.
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Get the number of bytes remaining.
    pub const fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Check if all data has been consumed.
    pub const fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Get the underlying data slice.
    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Get a slice of remaining data.
    pub fn remaining_data(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    /// Skip n bytes, returning an error if not enough data.
    pub fn skip(&mut self, n: usize) -> Result<(), ZeroCopyError> {
        if self.remaining() < n {
            return Err(ZeroCopyError::UnexpectedEof {
                needed: n,
                remaining: self.remaining(),
            });
        }
        self.pos += n;
        Ok(())
    }

    /// Read a fixed number of bytes as a reference.
    ///
    /// This is zero-copy - the returned slice references the original buffer.
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], ZeroCopyError> {
        if self.remaining() < n {
            return Err(ZeroCopyError::UnexpectedEof {
                needed: n,
                remaining: self.remaining(),
            });
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    /// Read a fixed-size array reference.
    ///
    /// Zero-copy: returns a reference to the underlying buffer.
    pub fn read_fixed<const N: usize>(&mut self) -> Result<&'a [u8; N], ZeroCopyError> {
        let bytes = self.read_bytes(N)?;
        // SAFETY: read_bytes guarantees exactly N bytes
        Ok(bytes.try_into().unwrap())
    }

    /// Read a u8.
    pub fn read_u8(&mut self) -> Result<u8, ZeroCopyError> {
        let bytes = self.read_fixed::<1>()?;
        Ok(bytes[0])
    }

    /// Read a u16 (little-endian).
    pub fn read_u16(&mut self) -> Result<u16, ZeroCopyError> {
        let bytes = self.read_fixed::<2>()?;
        Ok(u16::from_le_bytes(*bytes))
    }

    /// Read a u32 (little-endian).
    pub fn read_u32(&mut self) -> Result<u32, ZeroCopyError> {
        let bytes = self.read_fixed::<4>()?;
        Ok(u32::from_le_bytes(*bytes))
    }

    /// Read a u64 (little-endian).
    pub fn read_u64(&mut self) -> Result<u64, ZeroCopyError> {
        let bytes = self.read_fixed::<8>()?;
        Ok(u64::from_le_bytes(*bytes))
    }

    /// Read a u128 (little-endian).
    pub fn read_u128(&mut self) -> Result<u128, ZeroCopyError> {
        let bytes = self.read_fixed::<16>()?;
        Ok(u128::from_le_bytes(*bytes))
    }

    /// Read an i8.
    pub fn read_i8(&mut self) -> Result<i8, ZeroCopyError> {
        Ok(self.read_u8()? as i8)
    }

    /// Read an i16 (little-endian).
    pub fn read_i16(&mut self) -> Result<i16, ZeroCopyError> {
        let bytes = self.read_fixed::<2>()?;
        Ok(i16::from_le_bytes(*bytes))
    }

    /// Read an i32 (little-endian).
    pub fn read_i32(&mut self) -> Result<i32, ZeroCopyError> {
        let bytes = self.read_fixed::<4>()?;
        Ok(i32::from_le_bytes(*bytes))
    }

    /// Read an i64 (little-endian).
    pub fn read_i64(&mut self) -> Result<i64, ZeroCopyError> {
        let bytes = self.read_fixed::<8>()?;
        Ok(i64::from_le_bytes(*bytes))
    }

    /// Read an i128 (little-endian).
    pub fn read_i128(&mut self) -> Result<i128, ZeroCopyError> {
        let bytes = self.read_fixed::<16>()?;
        Ok(i128::from_le_bytes(*bytes))
    }

    /// Read a bool (0 = false, non-zero = true).
    pub fn read_bool(&mut self) -> Result<bool, ZeroCopyError> {
        Ok(self.read_u8()? != 0)
    }

    /// Read a Borsh-encoded string (u32 length prefix + UTF-8 bytes).
    ///
    /// This is NOT zero-copy because we need to validate UTF-8.
    pub fn read_string(&mut self) -> Result<String, ZeroCopyError> {
        let str_ref = self.read_str()?;
        Ok(str_ref.to_owned())
    }

    /// Read a Borsh-encoded string as a reference.
    ///
    /// This IS zero-copy but requires valid UTF-8 in the source buffer.
    pub fn read_str(&mut self) -> Result<&'a str, ZeroCopyError> {
        let len = self.read_u32()? as usize;

        // Sanity check to prevent reading gigabytes
        if len > self.remaining() {
            return Err(ZeroCopyError::LengthTooLarge { length: len });
        }

        let bytes = self.read_bytes(len)?;
        std::str::from_utf8(bytes).map_err(|_| ZeroCopyError::InvalidUtf8)
    }

    /// Read a Borsh Option<T> where T is fixed-size.
    ///
    /// Returns `Some(bytes)` if present, `None` otherwise.
    pub fn read_option_fixed<const N: usize>(
        &mut self,
    ) -> Result<Option<&'a [u8; N]>, ZeroCopyError> {
        let present = self.read_bool()?;
        if present {
            Ok(Some(self.read_fixed()?))
        } else {
            Ok(None)
        }
    }

    /// Peek at bytes without advancing the cursor.
    pub fn peek(&self, n: usize) -> Option<&'a [u8]> {
        if self.remaining() >= n {
            Some(&self.data[self.pos..self.pos + n])
        } else {
            None
        }
    }

    /// Peek at a fixed array without advancing the cursor.
    pub fn peek_fixed<const N: usize>(&self) -> Option<&'a [u8; N]> {
        self.peek(N).map(|s| s.try_into().unwrap())
    }

    /// Reset the cursor to the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Set the cursor to a specific position.
    pub fn seek(&mut self, pos: usize) -> Result<(), ZeroCopyError> {
        if pos > self.data.len() {
            return Err(ZeroCopyError::UnexpectedEof {
                needed: pos,
                remaining: self.data.len(),
            });
        }
        self.pos = pos;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_primitives() {
        let data = [
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // u64: 1
            0xFF, // u8: 255
            0x10, 0x00, 0x00, 0x00, // u32: 16
            0x01, // bool: true
            0x00, // bool: false
        ];

        let mut reader = ZeroCopyReader::new(&data);

        assert_eq!(reader.read_u64().unwrap(), 1);
        assert_eq!(reader.read_u8().unwrap(), 255);
        assert_eq!(reader.read_u32().unwrap(), 16);
        assert!(reader.read_bool().unwrap());
        assert!(!reader.read_bool().unwrap());
        assert!(reader.is_empty());
    }

    #[test]
    fn test_read_string() {
        let data = [
            0x05, 0x00, 0x00, 0x00, // length: 5
            b'h', b'e', b'l', b'l', b'o', // "hello"
        ];

        let mut reader = ZeroCopyReader::new(&data);
        assert_eq!(reader.read_str().unwrap(), "hello");
    }

    #[test]
    fn test_read_fixed() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8];

        let mut reader = ZeroCopyReader::new(&data);
        let fixed: &[u8; 4] = reader.read_fixed().unwrap();
        assert_eq!(fixed, &[1, 2, 3, 4]);

        let fixed: &[u8; 4] = reader.read_fixed().unwrap();
        assert_eq!(fixed, &[5, 6, 7, 8]);
    }

    #[test]
    fn test_eof_error() {
        let data = [1, 2, 3];

        let mut reader = ZeroCopyReader::new(&data);
        let result = reader.read_u64();

        assert!(matches!(
            result,
            Err(ZeroCopyError::UnexpectedEof {
                needed: 8,
                remaining: 3
            })
        ));
    }

    #[test]
    fn test_peek_and_skip() {
        let data = [1, 2, 3, 4, 5];

        let mut reader = ZeroCopyReader::new(&data);

        assert_eq!(reader.peek(2), Some(&[1, 2][..]));
        assert_eq!(reader.position(), 0);

        reader.skip(2).unwrap();
        assert_eq!(reader.position(), 2);

        assert_eq!(reader.peek(2), Some(&[3, 4][..]));
    }

    #[test]
    fn test_seek_and_reset() {
        let data = [1, 2, 3, 4, 5];

        let mut reader = ZeroCopyReader::new(&data);

        reader.skip(3).unwrap();
        assert_eq!(reader.position(), 3);

        reader.reset();
        assert_eq!(reader.position(), 0);

        reader.seek(4).unwrap();
        assert_eq!(reader.position(), 4);
    }
}
