use crate::http::request::RequestBuilder;
use crate::http::Http2Client;

#[test]
fn foreknowledge_get() {
    let request = RequestBuilder::get("http2.pro/api/v1");
    let mut client = Http2Client::new();
    let mut results = Vec::with_capacity(100);
    for _ in 0..101 {
        let start = std::time::Instant::now();
        let response = client.execute(request.clone()).unwrap();
        let end = std::time::Instant::now().duration_since(start);
        assert_eq!(response.status_code, 200);
        results.push(end);
    }
    for (i, end) in results.iter().enumerate() {
        println!("Run {i} Total Time: {end:#?}");
    }
    let avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {avg}");
}

#[test]
fn dummy_get() {
    let mut request = RequestBuilder::get("dummyapi.io/data/v1/user");
    request.headers(vec![(b"app-id", b"623e3f74a76d8facdad7758b")]);
    let mut client = Http2Client::new();
    let mut results = Vec::with_capacity(20);
    for _ in 0..21 {
        let start = std::time::Instant::now();
        let response = client.execute(request.clone()).unwrap();
        let end = std::time::Instant::now().duration_since(start);
        assert_eq!(response.status_code, 200);
        results.push(end);
    }
    for (i, end) in results.iter().enumerate() {
        println!("Run {i} Total Time: {end:#?}");
    }
    let avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {avg}");
}

#[test]
fn dummy_put() {
    let mut request = RequestBuilder::get("dummyapi.io/data/v1/user");
    request.headers(vec![(b"app-id", b"623e3f74a76d8facdad7758b")]);
    let mut client = Http2Client::new();
    let mut results = Vec::with_capacity(20);
    for _ in 0..21 {
        let start = std::time::Instant::now();
        let response = client.execute(request.clone()).unwrap();
        let end = std::time::Instant::now().duration_since(start);
        assert_eq!(response.status_code, 200);
        results.push(end);
    }
    for (i, end) in results.iter().enumerate() {
        println!("Run {i} Total Time: {end:#?}");
    }
    let avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {avg}");
}
