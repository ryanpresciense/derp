extern crate untrusted;

use std::io::{self, Write};

mod der;

pub use der::*;

pub enum Error {
    LeadingZero,
    LessThanMinimum,
    LongLengthNotSupported,
    HighTagNumberForm,
    Io,
    NegativeValue,
    NonCanonical,
    NonZeroUnusedBits,
    Read,
    UnexpectedEnd,
    WrongTag,
}

impl From<untrusted::EndOfInput> for Error {
    fn from(_: untrusted::EndOfInput) -> Error {
        Error::UnexpectedEnd
    }
}

impl From<io::Error> for Error {
    fn from(_: io::Error) -> Error {
        Error::Io
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;

pub struct Der<'a, W: Write + 'a> {
    writer: &'a mut W,
}

impl<'a, W: Write> Der<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Der { writer: writer }
    }

    fn length_of_length(len: usize) -> u8 {
        let mut i = len;
        let mut num_bytes = 1;

        while i > 255 {
            num_bytes += 1;
            i >>= 8;
        }

        num_bytes
    }

    fn write_len(&mut self, len: usize) -> Result<()> {
        if len >= 128 {
            let n = Self::length_of_length(len);
            self.writer.write_all(&[0x80 | n])?;

            for i in (1..n + 1).rev() {
                self.writer.write_all(&[(len >> ((i - 1) * 8)) as u8])?;
            }
        } else {
            self.writer.write_all(&[len as u8])?;
        }

        Ok(())
    }

    pub fn write_null(&mut self) -> Result<()> {
        Ok(self.writer.write_all(&[Tag::Null as u8, 0])?)
    }

    pub fn write_element(&mut self, tag: Tag, input: untrusted::Input) -> Result<()> {
        self.writer.write_all(&[tag as u8])?;
        let mut buf = Vec::new();

        input.read_all(Error::Read, |read| {
            while let Ok(byte) = read.read_byte() {
                buf.push(byte);
            }

            Ok(())
        })?;

        self.write_len(buf.len())?;

        Ok(self.writer.write_all(&mut buf)?)
    }

    pub fn write_integer(&mut self, input: untrusted::Input) -> Result<()> {
        self.writer.write_all(&[Tag::Integer as u8])?;
        let mut buf = Vec::new();

        input.read_all(Error::Read, |read| {
            while let Ok(byte) = read.read_byte() {
                buf.push(byte);
            }

            Ok(())
        })?;

        self.write_len(buf.len())?;

        Ok(self.writer.write_all(&mut buf)?)
    }

    pub fn write_sequence<F: FnOnce(&mut Der<Vec<u8>>) -> Result<()>>(
        &mut self,
        func: F,
    ) -> Result<()> {
        self.writer.write_all(&[Tag::Sequence as u8])?;
        let mut buf = Vec::new();

        {
            let mut inner = Der::new(&mut buf);
            func(&mut inner)?;
        }

        self.write_len(buf.len())?;
        Ok(self.writer.write_all(&buf)?)
    }

    pub fn write_raw(&mut self, input: untrusted::Input) -> Result<()> {
        Ok(self.writer.write_all(input.as_slice_less_safe())?)
    }

    pub fn write_bit_string<F: FnOnce(&mut Der<Vec<u8>>) -> Result<()>>(
        &mut self,
        func: F,
    ) -> Result<()> {
        self.writer.write_all(&[Tag::BitString as u8])?;
        let mut buf = Vec::new();
        // push 0x00 byte to say "no unused bits"
        buf.push(0x00);

        {
            let mut inner = Der::new(&mut buf);
            func(&mut inner)?;
        }

        self.write_len(buf.len())?;
        Ok(self.writer.write_all(&buf)?)
    }
}
