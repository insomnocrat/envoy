pub mod codec;
pub mod frames;
pub mod request;
pub mod stream;
#[cfg(test)]
mod tests;

pub use frames::*;

pub const NO_ERROR: u8 = 0x00;
pub const PROTOCOL_ERROR: u8 = 0x01;
pub const INTERNAL_ERROR: u8 = 0x02;
pub const FLOW_CONTROL_ERROR: u8 = 0x03;
pub const SETTINGS_TIMEOUT: u8 = 0x04;
pub const STREAM_CLOSED: u8 = 0x05;
pub const FRAME_SIZE_ERROR: u8 = 0x06;
pub const REFUSED_STREAM: u8 = 0x7;
pub const CANCEL: u8 = 0x8;
pub const COMPRESSION_ERROR: u8 = 0x9;
pub const CONNECT_ERROR: u8 = 0xa;
pub const ENHANCE_YOUR_CALM: u8 = 0xb;
pub const INADEQUATE_SECURITY: u8 = 0xc;
pub const HTTP_1_1_REQUIRED: u8 = 0xd;
