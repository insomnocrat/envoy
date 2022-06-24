use crate::http::request::RequestBuilder;
use crate::http::{HttpClient, Protocol, Response};

pub(crate) fn print_results(results: Vec<std::time::Duration>) {
    for (_i, _end) in results.iter().enumerate() {
        println!("Run {_i} Total Time: {_end:#?}");
    }
    let _avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {_avg}");
}

pub(crate) fn gen_test_request(res: &str) -> RequestBuilder {
    let request = RequestBuilder::get(&format!("jsonplaceholder.typicode.com/{res}"));
    match request.protocol {
        Protocol::HTTP1 => {
            request.headers(vec![(b"Accept", b"/*/"), (b"Connection", b"keep-alive")])
        }
        #[cfg(feature = "http2")]
        Protocol::HTTP2 => request,
    }
}

pub(crate) fn gen_test_request_2(res: &str) -> RequestBuilder {
    let request = RequestBuilder::get(&format!("dummyapi.io/data/v1/{res}"))
        .headers(vec![(b"app-id", b"623e3f74a76d8facdad7758b")]);

    request
}

pub(crate) fn iterate_request(
    client: &mut HttpClient,
    request: RequestBuilder,
    repeat: usize,
    evaluation: Option<Box<dyn Fn(Response) -> bool>>,
) {
    client.connect(&request.url).unwrap();
    let evaluation = match evaluation {
        Some(e) => e,
        None => {
            Box::new(|response: Response| response.status_code >= 200 && response.status_code < 302)
        }
    };
    let mut results = Vec::with_capacity(repeat);
    for _ in 0..repeat + 1 {
        let start = std::time::Instant::now();
        let response = client.execute(request.clone()).unwrap();
        let end = std::time::Instant::now().duration_since(start);
        assert!(evaluation(response));
        results.push(end);
    }
    print_results(results);
}
