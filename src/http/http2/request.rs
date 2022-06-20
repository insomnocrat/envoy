use crate::http::request::RequestBuilder;
use crate::http::Method;

#[derive(Debug, Clone)]
pub struct Request {
    pub raw_headers: Vec<(Vec<u8>, Vec<u8>)>,
    pub data: Option<Vec<u8>>,
}

impl Request {
    fn default_headers(
        method: Method,
        authority: &[u8],
        resource: &[u8],
        query: Vec<(Vec<u8>, Vec<u8>)>,
        custom_capacity: usize,
    ) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut headers = Vec::with_capacity(custom_capacity + 5);
        let method: (Vec<u8>, Vec<u8>) = match method {
            Method::GET => to_owned_header(headers::GET),
            Method::POST => to_owned_header(headers::POST),
            Method::PUT => to_owned_header(headers::PUT),
            Method::PATCH => to_owned_header(headers::PATCH),
            Method::DELETE => to_owned_header(headers::DELETE),
        };
        let mut resource = match resource.is_empty() {
            true => b"/".to_vec(),
            false => resource.to_vec(),
        };
        if !query.is_empty() {
            resource.push(0x3f);
            for (key, value) in query.into_iter() {
                resource.extend(key);
                resource.push(0x3D);
                resource.extend(value);
            }
        }
        headers.extend(vec![
            method,
            (headers::PATH.to_vec(), resource),
            to_owned_header(headers::SCHEME_HTTPS),
            (headers::AUTHORITY.to_vec(), authority.to_vec()),
        ]);

        headers
    }
}

impl From<RequestBuilder> for Request {
    fn from(builder: RequestBuilder) -> Self {
        let mut headers = Self::default_headers(
            builder.method,
            &builder.url.host,
            &builder.url.resource,
            builder.query,
            builder.headers.len(),
        );
        headers.extend(builder.headers.into_iter());

        Self {
            raw_headers: headers,
            data: builder.body,
        }
    }
}

pub mod headers {
    pub const METHOD: &[u8] = b":method";
    pub const GET: (&[u8], &[u8]) = (METHOD, b"GET");
    pub const POST: (&[u8], &[u8]) = (METHOD, b"POST");
    pub const PUT: (&[u8], &[u8]) = (METHOD, b"PUT");
    pub const PATCH: (&[u8], &[u8]) = (METHOD, b"PATCH");
    pub const DELETE: (&[u8], &[u8]) = (METHOD, b"DELETE");
    pub const AUTHORITY: &[u8] = b":authority";
    pub const PATH: &[u8] = b":path";
    pub const SCHEME: &[u8] = b":scheme";
    pub const SCHEME_HTTPS: (&[u8], &[u8]) = (SCHEME, b"https");
}

fn to_owned_header(header: (&[u8], &[u8])) -> (Vec<u8>, Vec<u8>) {
    let (key, value) = header;

    (key.to_vec(), value.to_vec())
}
