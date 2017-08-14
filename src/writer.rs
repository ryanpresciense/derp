use std::io::Write;

use Result;
use der::{self, Tag};

/// Helper for writing DER that automattically encoes tags and content lengths.
pub struct Der<'a, W: Write + 'a> {
    writer: &'a mut W,
}

impl<'a, W: Write> Der<'a, W> {
    /// Create a new `Der` structure that writes values to the given writer.
    pub fn new(writer: &'a mut W) -> Self {
        Der { writer: writer }
    }

    fn write_len(&mut self, len: usize) -> Result<()> {
        if len >= 128 {
            let n = der::length_of_length(len);
            self.writer.write_all(&[0x80 | n])?;

            for i in (1..n + 1).rev() {
                self.writer.write_all(&[(len >> ((i - 1) * 8)) as u8])?;
            }
        } else {
            self.writer.write_all(&[len as u8])?;
        }

        Ok(())
    }

    /// Write a `NULL` tag.
    pub fn write_null(&mut self) -> Result<()> {
        Ok(self.writer.write_all(&[Tag::Null as u8, 0])?)
    }

    /// Write an arbitrary element.
    pub fn write_element(&mut self, tag: Tag, input: &[u8]) -> Result<()> {
        self.writer.write_all(&[tag as u8])?;
        self.write_len(input.len())?;
        self.writer.write_all(input)?;
        Ok(())
    }

    /// Write the given input as an integer.
    pub fn write_integer(&mut self, input: &[u8]) -> Result<()> {
        self.writer.write_all(&[Tag::Integer as u8])?;
        self.write_len(input.len())?;
        self.writer.write_all(input)?;
        Ok(())
    }

    /// Write the given input as a positive integer.
    pub fn write_positive_integer(&mut self, input: &[u8]) -> Result<()> {
        self.writer.write_all(&[Tag::Integer as u8])?;

        let push_zero = if input.len() > 0 {
            input[0] & 0x80 == 0x80
        } else {
            false
        };

        self.write_len(input.len() + push_zero as usize)?;
        
        if push_zero {
            self.writer.write_all(&[0x00])?;
        }

        self.writer.write_all(input)?;
        Ok(())
    }

    /// Write a `SEQUENCE` by passing in a handling function that writes to an intermediate `Vec`
    /// before writing the whole sequence to `self`.
    pub fn write_sequence<F: FnOnce(&mut Der<Vec<u8>>) -> Result<()>>(
        &mut self,
        func: F,
    ) -> Result<()> {
        let mut buf = Vec::new();

        {
            let mut inner = Der::new(&mut buf);
            func(&mut inner)?;
        }

        self.writer.write_all(&[Tag::Sequence as u8])?;
        self.write_len(buf.len())?;
        Ok(self.writer.write_all(&buf)?)
    }

    /// Write an `OBJECT IDENTIFIER`.
    pub fn write_oid(&mut self, input: &[u8]) -> Result<()> {
        self.writer.write_all(&[Tag::Oid as u8])?;
        self.write_len(input.len())?;
        self.writer.write_all(&input)?;
        Ok(())
    }

    /// Write raw bytes to `self`. This does not calculate length or apply. This should only be used
    /// when you know you are dealing with bytes that are already DER encoded.
    pub fn write_raw(&mut self, input: &[u8]) -> Result<()> {
        Ok(self.writer.write_all(input)?)
    }

    /// Write a `BIT STRING` by passing in a handling function that writes to an intermediate `Vec`
    /// before writing the whole sequence to `self`.
    pub fn write_bit_string<F: FnOnce(&mut Der<Vec<u8>>) -> Result<()>>(
        &mut self,
        unused_bits: u8,
        func: F,
    ) -> Result<()> {
        let mut buf = Vec::new();
        buf.push(unused_bits);

        {
            let mut inner = Der::new(&mut buf);
            func(&mut inner)?;
        }

        self.writer.write_all(&[Tag::BitString as u8])?;
        self.write_len(buf.len())?;
        Ok(self.writer.write_all(&buf)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use Error;
    use untrusted::Input;
    
    static RSA_2048_PKCS1: &'static [u8] = include_bytes!("../tests/rsa-2048.pkcs1.der");

    #[test]
    fn test_write_pkcs1() {
        let input = Input::from(RSA_2048_PKCS1);
        let (n, e) = input.read_all(Error::Read, |input| {
            der::nested(input, Tag::Sequence, |input| {
                let n = der::positive_integer(input)?;
                let e = der::positive_integer(input)?;
                Ok((n.as_slice_less_safe(), e.as_slice_less_safe()))
            })
        }).unwrap();

        let mut buf = Vec::new();
        {
            let mut der = Der::new(&mut buf);
            der.write_sequence(|der| {
                der.write_positive_integer(n)?;
                der.write_positive_integer(e)
            }).unwrap();
        }

        assert_eq!(buf.as_slice(), RSA_2048_PKCS1);
    }
}
