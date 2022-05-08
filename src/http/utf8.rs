use std::borrow::Cow;
use std::string::FromUtf8Error;

pub const NULL: u8 = 0x00;
pub const LF: u8 = 0x0a;
pub const CR: u8 = 0x0d;
pub const SP: u8 = 0x20;
pub const CHUNK_END: &str = "\r\n0\r\n\r\n";
pub const FINAL_CHUNK: &[u8] = b"0\r\n\r\n";
pub const CRLF: &[u8; 2] = b"\r\n";
pub const DANGLING_CHUNK: &[u8; 3] = b"\r\n0";
pub const HEX_DIGITS: [u8; 22] = [
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46,
    0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
];

pub trait UTF8 {
    fn utf8(&self) -> Result<String, FromUtf8Error>;
    fn utf8_lossy(&self) -> Cow<str>;
    fn print_utf8(&self);
    fn debug_utf8(&self);
    fn is_hex(&self) -> bool;
}

impl UTF8 for Vec<u8> {
    fn utf8(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.clone())
    }
    fn utf8_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(self)
    }
    fn print_utf8(&self) {
        println!("{}", self.utf8_lossy());
    }
    fn debug_utf8(&self) {
        println!("{:?}", self.utf8_lossy());
    }
    fn is_hex(&self) -> bool {
        self.iter().all(|c| HEX_DIGITS.contains(c))
    }
}

impl UTF8 for &[u8] {
    fn utf8(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.to_vec())
    }
    fn utf8_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(&self)
    }
    fn print_utf8(&self) {
        println!("{}", self.utf8_lossy());
    }
    fn debug_utf8(&self) {
        println!("{:?}", self.utf8_lossy());
    }
    fn is_hex(&self) -> bool {
        self.iter().all(|c| HEX_DIGITS.contains(c))
    }
}
