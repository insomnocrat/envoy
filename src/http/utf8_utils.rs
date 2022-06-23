use std::borrow::Cow;
use std::io::Read;
use std::iter::Peekable;
use std::ops::Deref;
use std::string::FromUtf8Error;
use std::vec::IntoIter;

pub const NULL: u8 = 0x00;
pub const LF: u8 = 0x0a;
pub const CR: u8 = 0x0d;
pub const CRLF: &[u8; 2] = &[CR, LF];
pub const COLON: u8 = 0x3a;
pub const SP: u8 = 0x20;
pub const COLSP: &[u8; 2] = &[COLON, SP];
pub const SLASH: u8 = 0x2f;
pub const QMARK: u8 = 0x3f;
pub const EQUALS: u8 = 0x3d;

pub const HEX_DIGITS: &[u8; 22] = &[
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46,
    0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
];
pub const CAPITALS: &[u8; 26] = &[
    0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50,
    0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a,
];

pub trait UTF8Utils {
    fn as_utf8(&self) -> Result<String, FromUtf8Error>;
    fn as_utf8_lossy(&self) -> Cow<str>;
    fn print_utf8(&self);
    fn debug_utf8(&self);
    fn is_hex(&self) -> bool;
    fn as_lower(&self) -> Vec<u8>;
    fn to_lower(self) -> Vec<u8>;
    fn strip_null(self) -> Vec<u8>;
    fn trim_crlf(self) -> Vec<u8>;
    fn trim_chars(self, chars: &[u8]) -> Vec<u8>;
    fn into_utf8_parser(self) -> UTF8Parser;
}

impl<T: Deref<Target = [u8]>> UTF8Utils for T {
    fn as_utf8(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.as_ref().to_vec())
    }

    fn as_utf8_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(self.as_ref())
    }

    fn print_utf8(&self) {
        println!("{}", self.as_utf8_lossy());
    }

    fn debug_utf8(&self) {
        println!("{:?}", self.as_utf8_lossy());
    }

    fn is_hex(&self) -> bool {
        self.iter().all(|c| HEX_DIGITS.contains(c))
    }

    fn as_lower(&self) -> Vec<u8> {
        self.as_ref()
            .into_iter()
            .map(|c| match CAPITALS.contains(c) {
                true => c + 32,
                false => *c,
            })
            .collect()
    }

    fn to_lower(self) -> Vec<u8> {
        self.into_iter()
            .map(|c| match CAPITALS.contains(c) {
                true => c + 32,
                false => *c,
            })
            .collect()
    }

    fn strip_null(self) -> Vec<u8> {
        self.into_iter()
            .filter(|b| **b != NULL)
            .map(|b| *b)
            .collect()
    }
    fn trim_crlf(self) -> Vec<u8> {
        let mut vec = self.to_vec();
        while vec.ends_with(CRLF) && !vec.is_empty() {
            vec.pop();
            vec.pop();
        }

        vec
    }

    fn trim_chars(self, chars: &[u8]) -> Vec<u8> {
        let mut vec = self.to_vec();
        while vec.ends_with(chars) && !vec.is_empty() {
            for _ in 0..chars.len() {
                vec.pop();
            }
            break;
        }

        vec
    }

    fn into_utf8_parser(self) -> UTF8Parser {
        UTF8Parser::new(self.to_vec().as_slice())
    }
}

#[derive(Debug, Clone)]
pub struct UTF8Parser {
    iter: Peekable<IntoIter<u8>>,
}

impl<T: UTF8Utils> From<T> for UTF8Parser {
    fn from(utf8: T) -> Self {
        utf8.into_utf8_parser()
    }
}

impl UTF8Parser {
    pub fn new(buffer: &[u8]) -> Self {
        let iter = buffer.strip_null().into_iter().peekable();

        Self { iter }
    }
    fn load_buf(&mut self, buf: &mut [u8], bytes: &[u8]) -> std::io::Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        let read = self.calc_read(&buf, bytes);
        for num in 0..read {
            buf[num] = bytes[num];
        }

        Ok(read)
    }
    pub fn read_to_crlf(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let mut read = 0;
        while let Some(byte) = self.iter.next() {
            if byte == CR {
                if self.iter.next_if(|b| *b == LF).is_some() {
                    break;
                }
            }
            buf.push(byte);
            read += 1;
        }

        Ok(read)
    }

    pub fn take_crlf_strings(&mut self) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = Vec::new();
        while let Ok(num) = self.read_to_crlf(&mut line) {
            if num == 0 {
                break;
            }
            lines.push(line.as_utf8_lossy().to_string());
            line.clear();
        }

        lines
    }

    pub fn take_crlf(&mut self) -> Vec<Vec<u8>> {
        let mut lines = Vec::new();
        let mut line = Vec::new();
        while let Ok(num) = self.read_to_crlf(&mut line) {
            if num == 0 {
                break;
            }
            lines.push(line);
            line = Vec::new();
        }

        lines
    }

    pub fn read_to_char(&mut self, char: &u8, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let mut read = 0;
        while let Some(byte) = self.iter.next() {
            if byte == *char {
                break;
            }
            buf.push(byte);
            read += 1;
        }

        Ok(read)
    }

    pub fn read_to_space(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.read_to_char(&SP, buf)
    }

    pub fn read_to_lf(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.read_to_char(&LF, buf)
    }
    pub fn read_to_null(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.read_to_char(&NULL, buf)
    }
    pub fn skip_chars(&mut self, chars: &[u8]) {
        while self.iter.next_if(|c| chars.contains(c)).is_some() {}
    }
    pub fn skip_to_crlf(&mut self) {
        while let Some(byte) = self.iter.next() {
            if byte == CR {
                if self.iter.next_if(|b| *b == LF).is_some() {
                    break;
                }
            }
        }
    }
    fn calc_read(&self, buf: &[u8], bytes: &[u8]) -> usize {
        bytes.len() - buf.len()
    }
    pub fn to_vec(self) -> Vec<u8> {
        self.iter.collect::<Vec<u8>>()
    }
    pub fn take_hex(&mut self) -> Vec<u8> {
        let mut hex = Vec::new();
        while let Some(char) = self.iter.next_if(|c| HEX_DIGITS.contains(c)) {
            hex.push(char);
        }

        hex
    }
}

impl Read for UTF8Parser {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut bytes = Vec::with_capacity(self.iter.len());
        while let Some(byte) = self.iter.next() {
            bytes.push(byte);
        }

        self.load_buf(buf, &bytes)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        self.read_to_null(buf)
    }
    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        let mut bytes = Vec::new();
        self.read_to_crlf(&mut bytes)?;
        let size = bytes.len();
        if buf.capacity() - buf.len() < size {
            buf.reserve_exact(size);
        }
        buf.push_str(&bytes.as_utf8_lossy());

        Ok(size)
    }
}
