use crate::http::request::RequestBuilder;
use crate::http::test_utils::*;
use crate::http::HttpClient;

#[test]
fn foreknowledge_get() {
    let request = RequestBuilder::get("http2.pro/api/v1");
    let mut client = HttpClient::new();
    iterate_request(&mut client, request, 100, None);
}

#[test]
fn dummy_get() {
    let mut request = RequestBuilder::get("dummyapi.io/data/v1/user");
    request.headers(vec![(b"app-id", b"623e3f74a76d8facdad7758b")]);
    let mut client = HttpClient::new();
    iterate_request(&mut client, request, 20, None);
}

#[test]
fn dummy_put() {
    let mut request = RequestBuilder::get("dummyapi.io/data/v1/user");
    request.headers(vec![(b"app-id", b"623e3f74a76d8facdad7758b")]);
    let mut client = HttpClient::new();
    iterate_request(&mut client, request, 20, None);
}
