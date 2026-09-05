//! Slice-based Borsh utilities.
//!
//! Provides utilities for working with Borsh-encoded data as slices,
//! supporting both owned and borrowed data patterns.

use borsh::BorshDeserialize;
use std::io::Read;

/// A Borsh-encoded slice that can be deserialized on demand.
///
/// This is useful when you want to defer deserialization or
/// only deserialize if certain conditions are met.
#[derive(Debug, Clone, Copy)]
pub struct BorshSlice<'a> {
    data: &'a [u8],
}

impl<'a> BorshSlice<'a> {
    /// Create a new BorshSlice from a byte slice.
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Get the underlying bytes.
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.data
    }

    /// Get the length in bytes.
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the slice is empty.
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Deserialize a value from the start of the slice.
    ///
    /// Reads only as many bytes as `T` needs and ignores any trailing bytes.
    /// This is deliberate: account data is routinely a known struct followed by
    /// padding, extensions or unrelated tail bytes, so requiring the slice to
    /// be exactly the size of `T` would make this unusable on the decode path.
    /// Use [`deserialize_exact`](Self::deserialize_exact) when the whole slice
    /// really must be consumed.
    pub fn deserialize<T: BorshDeserialize>(&self) -> Result<T, borsh::io::Error> {
        let mut cursor = self.data;
        T::deserialize(&mut cursor)
    }

    /// Deserialize a value and require that the entire slice was consumed.
    ///
    /// Useful when the length is itself a correctness signal, e.g. a fixed-size
    /// account whose trailing bytes would indicate a different account type.
    pub fn deserialize_exact<T: BorshDeserialize>(&self) -> Result<T, borsh::io::Error> {
        T::try_from_slice(self.data)
    }

    /// Deserialize from a specific offset, ignoring trailing bytes.
    pub fn deserialize_at<T: BorshDeserialize>(
        &self,
        offset: usize,
    ) -> Result<T, borsh::io::Error> {
        if offset >= self.data.len() {
            return Err(borsh::io::Error::new(
                borsh::io::ErrorKind::UnexpectedEof,
                "offset exceeds data length",
            ));
        }
        let mut cursor = &self.data[offset..];
        T::deserialize(&mut cursor)
    }

    /// Split at an offset, returning two slices.
    pub fn split_at(&self, offset: usize) -> Option<(BorshSlice<'a>, BorshSlice<'a>)> {
        if offset > self.data.len() {
            return None;
        }
        let (left, right) = self.data.split_at(offset);
        Some((BorshSlice::new(left), BorshSlice::new(right)))
    }

    /// Skip the first n bytes.
    pub fn skip(&self, n: usize) -> Option<BorshSlice<'a>> {
        if n > self.data.len() {
            return None;
        }
        Some(BorshSlice::new(&self.data[n..]))
    }

    /// Take the first n bytes.
    pub fn take(&self, n: usize) -> Option<BorshSlice<'a>> {
        if n > self.data.len() {
            return None;
        }
        Some(BorshSlice::new(&self.data[..n]))
    }
}

impl<'a> AsRef<[u8]> for BorshSlice<'a> {
    fn as_ref(&self) -> &[u8] {
        self.data
    }
}

/// A reader that tracks how many bytes have been consumed.
///
/// Useful for parsing variable-length Borsh data where you need
/// to know where one value ends and the next begins.
#[derive(Debug)]
pub struct SliceReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> SliceReader<'a> {
    /// Create a new reader.
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Get the current position.
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Get remaining bytes.
    pub const fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Check if all data has been consumed.
    pub const fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Deserialize a value, advancing the position.
    pub fn read<T: BorshDeserialize>(&mut self) -> Result<T, borsh::io::Error> {
        let slice = &self.data[self.pos..];
        let mut cursor = std::io::Cursor::new(slice);

        let value = T::deserialize_reader(&mut cursor)?;
        self.pos += cursor.position() as usize;

        Ok(value)
    }

    /// Peek at remaining data without consuming.
    pub fn peek(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    /// Get a BorshSlice of remaining data.
    pub fn remaining_slice(&self) -> BorshSlice<'a> {
        BorshSlice::new(&self.data[self.pos..])
    }
}

impl<'a> Read for SliceReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let remaining = &self.data[self.pos..];
        let n = std::cmp::min(buf.len(), remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.pos += n;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_borsh_slice() {
        // A u32 (1) followed by a u8 (255)
        let data = [0x01, 0x00, 0x00, 0x00, 0xFF];

        let slice = BorshSlice::new(&data);
        assert_eq!(slice.len(), 5);

        let val: u32 = slice.deserialize().unwrap();
        assert_eq!(val, 1);

        let after_skip = slice.skip(4).unwrap();
        let val: u8 = after_skip.deserialize().unwrap();
        assert_eq!(val, 255);

        // Reading a prefix is fine; demanding exact consumption is not, because
        // one byte remains after the u32.
        assert!(slice.deserialize_exact::<u32>().is_err());
        assert_eq!(slice.deserialize_at::<u8>(4).unwrap(), 255);
    }

    #[test]
    fn test_slice_reader() {
        let data = [
            0x01, 0x00, 0x00, 0x00, // u32: 1
            0xFF, // u8: 255
        ];

        let mut reader = SliceReader::new(&data);

        let val: u32 = reader.read().unwrap();
        assert_eq!(val, 1);
        assert_eq!(reader.position(), 4);

        let val: u8 = reader.read().unwrap();
        assert_eq!(val, 255);
        assert_eq!(reader.position(), 5);

        assert!(reader.is_empty());
    }

    #[test]
    fn test_split_at() {
        let data = [1, 2, 3, 4, 5];
        let slice = BorshSlice::new(&data);

        let (left, right) = slice.split_at(2).unwrap();
        assert_eq!(left.as_bytes(), &[1, 2]);
        assert_eq!(right.as_bytes(), &[3, 4, 5]);
    }
}
