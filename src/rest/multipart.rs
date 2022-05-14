use crate::http::utf8_utils::{UTF8Utils, CRLF};
use rand;
use rand::distributions::Alphanumeric;
use rand::Rng;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct MultipartForm {
    pub boundary: Vec<u8>,
    pub data: Vec<Vec<u8>>,
}

impl MultipartForm {
    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = b"--".to_vec();
        bytes.extend_from_slice(&self.boundary);
        self.data.into_iter().for_each(|d| {
            bytes.extend(CRLF);
            bytes.extend(d);
            bytes.extend(CRLF);
            bytes.extend(b"--");
            bytes.extend_from_slice(&self.boundary);
        });
        bytes.extend(b"--");

        bytes
    }
}

impl<'a, T: AsRef<[u8]>> From<&[(T, T)]> for MultipartForm {
    fn from(input: &[(T, T)]) -> Self {
        let mut data = Vec::with_capacity(input.len());
        input
            .into_iter()
            .for_each(|(k, v)| data.push(to_form((k.as_ref(), v.as_ref()))));
        let mut boundary = b"".to_vec();
        let rob = rand::thread_rng().sample_iter(&Alphanumeric).take(6);
        boundary.extend(rob);

        Self { boundary, data }
    }
}

fn to_form(data: (&[u8], &[u8])) -> Vec<u8> {
    let (key, value) = data;
    let mut form = format!(
        "Content-Disposition: form-data; name=\"{}\"\r\nContent-Type; text/plain; charset=us-ascii\r\n\r\n",
        key.as_utf8_lossy()
    )
    .into_bytes();
    form.extend(value);
    form.debug_utf8();

    form
}
