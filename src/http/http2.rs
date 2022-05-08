mod codec;
pub(crate) mod frames;
pub(crate) mod stream;

pub use frames::*;

pub const GET: &[u8] = b"GET";
pub const POST: &[u8] = b"POST";
pub const PUT: &[u8] = b"PUT";
pub const PATCH: &[u8] = b"PATCH";
pub const DELETE: &[u8] = b"DELETE";
