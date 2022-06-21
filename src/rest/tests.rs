mod models;
use models::*;

use crate::http::test_utils::print_results;
use crate::rest::*;
use std::time;

fn dummy_api_client() -> Client {
    Client::config()
        .base_url("dummyapi.io")
        .required_header(("app-id", "623e3f74a76d8facdad7758b"))
        .preconnect()
        .unwrap()
}

#[test]
fn get_user() {
    let mut results = Vec::with_capacity(50);
    let mut client = dummy_api_client();
    for _ in 0..51 {
        let start = time::Instant::now();
        client.get("data/v1/user").send().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}

#[test]
fn get_user_json() {
    let mut results = Vec::with_capacity(50);
    let mut client = dummy_api_client();
    for _ in 0..51 {
        let start = time::Instant::now();
        let _response: UserList = client.get("data/v1/user").expect_json().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}

#[test]
fn put() {
    let mut results = Vec::with_capacity(10);
    let mut client = dummy_api_client();
    let user_list = client
        .get("data/v1/user")
        .send()
        .unwrap()
        .json::<UserList>()
        .unwrap();
    let resource = format!("data/v1/user/{}", &user_list.data[0].id);
    let user_id = user_list.data[0].id.to_string();
    let mut user = UserPreview::default();
    user.id = user_id;
    user.title = "Test".to_string();
    for _ in 0..11 {
        let start = time::Instant::now();
        client.put(&resource).body(&user).send().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}

#[test]
fn put_json() {
    let mut results = Vec::with_capacity(10);
    let mut client = dummy_api_client();
    let user_list = client
        .get("data/v1/user")
        .send()
        .unwrap()
        .json::<UserList>()
        .unwrap();
    let resource = format!("data/v1/user/{}", &user_list.data[0].id);
    let user_id = user_list.data[0].id.to_string();
    let mut user = UserPreview::default();
    user.id = user_id;
    user.title = "Test".to_string();
    for _ in 0..11 {
        let start = time::Instant::now();
        let _result: User = client.put(&resource).body(&user).expect_json().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}

#[test]
fn post_and_delete() {
    let mut results = Vec::with_capacity(10);
    let mut client = dummy_api_client();
    let mut user = User::default();
    user.first_name = "TestMan".to_string();
    user.last_name = "Testy".to_string();
    user.email = "random_email@ihatecomingupwithnewemails.com".to_string();
    user.date_of_birth = "1/1/1900".to_string();
    for _ in 0..11 {
        let start = time::Instant::now();
        let user = client
            .post("data/v1/user/create")
            .body(&user)
            .expect_json::<User>()
            .unwrap();
        let route = format!("data/v1/user/{}", user.id);
        client.delete(&route).send().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}

#[test]
#[cfg(not(feature = "http2"))]
pub fn query_test() {
    let mut results = Vec::with_capacity(10);
    let mut client = crate::RestClient::new("api.agify.io");
    for _ in 0..11 {
        let start = time::Instant::now();
        client.get("/").query(&[("name", "isaac")]).send().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}
#[cfg(not(feature = "http2"))]
#[derive(serde::Serialize, Clone)]
struct TestQuery {
    name: String,
}
#[test]
#[cfg(not(feature = "http2"))]
pub fn query_serialize_test() {
    let mut results = Vec::with_capacity(10);
    let mut client = crate::RestClient::new("api.agify.io");
    let query = TestQuery {
        name: "Isaac".to_string(),
    };
    for _ in 0..11 {
        let start = time::Instant::now();
        client.get("/").query(&query).send().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}
#[test]
#[cfg(not(feature = "http2"))]
pub fn opt_query_serialize_test() {
    let mut results = Vec::with_capacity(10);
    let mut client = crate::RestClient::new("api.agify.io");
    let query = Some(TestQuery {
        name: "Isaac".to_string(),
    });
    for _ in 0..11 {
        let start = time::Instant::now();
        client.get("/").opt_query(query.clone()).send().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}
#[test]
#[cfg(not(feature = "http2"))]
pub fn opt_query_serialize_test2() {
    let mut results = Vec::with_capacity(10);
    let mut client = crate::RestClient::new("api.agify.io");
    let query: Option<TestQuery> = None;
    for _ in 0..11 {
        let start = time::Instant::now();
        client.get("/").opt_query(query.clone()).send().unwrap_err();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}
