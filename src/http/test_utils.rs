use crate::http::request::RequestBuilder;
use crate::http::{HttpClient, Response};

pub(crate) fn print_results(results: Vec<std::time::Duration>) {
    for (_i, _end) in results.iter().enumerate() {
        println!("Run {_i} Total Time: {_end:#?}");
    }
    let _avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {_avg}");
}

pub(crate) fn gen_test_request(res: &str) -> RequestBuilder {
    let mut request = RequestBuilder::get(&format!("jsonplaceholder.typicode.com/{res}"));
    request.headers(vec![(b"Accept", b"/*/"), (b"Connection", b"keep-alive")]);

    request
}

pub(crate) fn gen_test_request_2(res: &str) -> RequestBuilder {
    let mut request = RequestBuilder::get(&format!("dummyapi.io/data/v1/{res}"));
    request.headers(vec![(b"app-id", b"623e3f74a76d8facdad7758b")]);

    request
}

pub(crate) fn iterate_request(
    client: &mut HttpClient,
    request: RequestBuilder,
    amount: usize,
    evaluation: Option<Box<dyn Fn(Response) -> bool>>,
) {
    let evaluation = match evaluation {
        Some(e) => e,
        None => Box::new(|response: Response| response.status_code == 200),
    };
    let mut results = Vec::with_capacity(amount);
    for _ in 0..amount + 1 {
        let start = std::time::Instant::now();
        let response = client.execute(request.clone()).unwrap();
        let end = std::time::Instant::now().duration_since(start);
        assert!(evaluation(response));
        results.push(end);
    }
    print_results(results);
}
