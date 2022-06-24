use crate::http::request::headers::{values::ALL, ACCEPT};
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
    let request = RequestBuilder::get("dummyapi.io/data/v1/user")
        .headers(vec![(b"app-id", b"623e3f74a76d8facdad7758b")]);
    let mut client = HttpClient::new();
    iterate_request(&mut client, request, 20, None);
}

#[test]
fn dummy_put() {
    let request = RequestBuilder::get("dummyapi.io/data/v1/user")
        .headers(vec![(b"app-id", b"623e3f74a76d8facdad7758b")]);
    let mut client = HttpClient::new();
    iterate_request(&mut client, request, 20, None);
}

#[test]
fn downgrade_test() {
    let mut client = HttpClient::new();
    iterate_request(
        &mut client,
        RequestBuilder::get(&format!("api.nationalize.io"))
            .query(vec![("name", "isaac")])
            .headers(vec![(ACCEPT, ALL)]),
        50,
        None,
    );
}

#[test]
fn ping() {
    let mut client = HttpClient::new();
    let url = "http2.pro/api/v1".into();
    client.connect(&url).unwrap();
    for _ in 0..25 {
        assert!(client.ping().is_ok());
    }
}
