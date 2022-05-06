use crate::http::request::{Request, RequestBuilder};
use crate::HttpClient;
use std::time;

fn gen_test_request(res: &str) -> Request {
    let mut request = RequestBuilder::get(&format!("jsonplaceholder.typicode.com/{res}"));
    request.headers(vec![(b"Accept", b"/*/"), (b"Connection", b"keep-alive")]);

    request.build()
}

#[test]
fn get_photos() {
    println!("Photos");
    let request = gen_test_request("photos");
    let mut client = HttpClient::new();
    let mut results = Vec::with_capacity(30);
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
    let mut results = Vec::with_capacity(30);
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
    let mut results = Vec::with_capacity(30);
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
