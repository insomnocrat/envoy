use crate::http::request::{Http1Request, RequestBuilder};
use crate::HttpClient;
use std::time;

fn gen_test_request(res: &str) -> Http1Request {
    let mut request = RequestBuilder::get(&format!("jsonplaceholder.typicode.com/{res}"));
    request.headers(vec![(b"Accept", b"/*/"), (b"Connection", b"keep-alive")]);

    request.build_http1()
}

fn gen_test_request_2(res: &str) -> Http1Request {
    let mut request = RequestBuilder::get(&format!("dummyapi.io/data/v1/{res}"));
    request.headers(vec![
        (b"Accept", b"/*/"),
        (b"Connection", b"keep-alive"),
        (b"app-id", b"623e3f74a76d8facdad7758b"),
    ]);

    request.build_http1()
}

#[test]
fn get_photos() {
    println!("Photos");
    let request = gen_test_request("photos");
    let mut client = HttpClient::new();
    let mut results = Vec::with_capacity(100);
    for _ in 0..101 {
        let start = time::Instant::now();
        let response = client.execute(request.clone()).unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
        assert_eq!(response.body.len(), 1071472)
    }
    for (i, end) in results.iter().enumerate() {
        println!("Run {i} Total Time: {end:#?}");
    }
    let avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {avg}");
}

#[test]
fn get_posts() {
    println!("Posts");
    let request = gen_test_request("posts");
    let mut client = HttpClient::new();
    let mut results = Vec::with_capacity(100);
    for _ in 0..101 {
        let start = time::Instant::now();
        let response = client.execute(request.clone()).unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
        assert_eq!(response.body.len(), 27520)
    }
    for (i, end) in results.iter().enumerate() {
        println!("Run {i} Total Time: {end:#?}");
    }
    let avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {avg}");
}
#[test]
fn get_users() {
    println!("Users");
    let request = gen_test_request("users");
    let mut client = HttpClient::new();
    let mut results = Vec::with_capacity(100);
    for _ in 0..101 {
        let start = time::Instant::now();
        let response = client.execute(request.clone()).unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
        assert_eq!(response.body.len(), 5645);
    }
    for (i, end) in results.iter().enumerate() {
        println!("Run {i} Total Time: {end:#?}");
    }
    let avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {avg}");
}

#[test]
fn get_users2() {
    println!("Users2");
    let request = gen_test_request_2("user");
    let mut client = HttpClient::new();
    let mut results = Vec::with_capacity(100);
    for _ in 0..101 {
        let start = time::Instant::now();
        let _ = client.execute(request.clone()).unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    for (i, end) in results.iter().enumerate() {
        println!("Run {i} Total Time: {end:#?}");
    }
    let avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {avg}");
}

// #[cfg(feature = "http2")]
// #[test]
// fn http2_test() {
//     let mut request = RequestBuilder::get("dummyapi.io/data/v1/user");
//     request.header((b"app-id", b"623e3f74a76d8facdad7758b"));
//     let request = request.build_http1();
//     let mut client = HttpClient::new();
//     let response = client.execute(request).unwrap();
//     println!("{:?}", response.headers);
//     response.body.print_utf8();
// }
