use crate::http::test_utils::*;
use crate::http::Response;
use crate::HttpClient;

#[test]
fn get_photos() {
    let evaluation = Box::new(|response: Response| response.body.len() == 1071472);
    let request = gen_test_request("photos");
    let mut client = HttpClient::new();
    iterate_request(&mut client, request, 100, Some(evaluation));
}

#[test]
fn get_posts() {
    let evaluation = Box::new(|response: Response| response.body.len() == 27520);
    let request = gen_test_request("posts");
    let mut client = HttpClient::new();
    iterate_request(&mut client, request, 100, Some(evaluation));
}
#[test]
fn get_users() {
    let evaluation = Box::new(|response: Response| response.body.len() == 5645);
    let request = gen_test_request("users");
    let mut client = HttpClient::new();
    iterate_request(&mut client, request, 100, Some(evaluation));
}

#[test]
fn get_users2() {
    let request = gen_test_request_2("user");
    let mut client = HttpClient::new();
    iterate_request(&mut client, request, 100, None);
}
