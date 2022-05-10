mod models;
use models::*;

use super::*;
use std::time;

fn dummy_api_client() -> Client {
    Client::from(
        ClientConfig::from("dummyapi.io").required_header(("app-id", "623e3f74a76d8facdad7758b")),
    )
    .preconnect()
    .unwrap()
}

fn print_results(results: Vec<time::Duration>) {
    for (_i, _end) in results.iter().enumerate() {
        println!("Run {_i} Total Time: {_end:#?}");
    }
    let _avg: f32 =
        (results.iter().map(|r| r.as_secs_f32()).sum::<f32>() / results.len() as f32) * 1000.0;
    println!("Avg. {_avg}");
}

#[test]
fn get_user() {
    let mut results = Vec::with_capacity(100);
    let mut client = dummy_api_client();
    for _ in 0..101 {
        let start = time::Instant::now();
        let response = client.get("data/v1/user").send().unwrap();
        let end = time::Instant::now().duration_since(start);
        assert!(response.is_ok());
        results.push(end);
    }
    print_results(results);
}

#[test]
fn get_user_json() {
    let mut results = Vec::with_capacity(100);
    let mut client = dummy_api_client();
    for _ in 0..101 {
        let start = time::Instant::now();
        let _response: UserList = client.get("data/v1/user").expect_json().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}

#[test]
fn put() {
    let mut results = Vec::with_capacity(100);
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
    for _ in 0..101 {
        let start = time::Instant::now();
        let result = client.put(&resource).body(&user).send().unwrap();
        let end = time::Instant::now().duration_since(start);
        assert!(result.is_ok());
        results.push(end);
    }
    print_results(results);
}

#[test]
fn put_json() {
    let mut results = Vec::with_capacity(100);
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
    for _ in 0..101 {
        let start = time::Instant::now();
        let _result: User = client.put(&resource).body(&user).expect_json().unwrap();
        let end = time::Instant::now().duration_since(start);
        results.push(end);
    }
    print_results(results);
}

#[test]
fn post_and_delete() {
    let mut results = Vec::with_capacity(100);
    let mut client = dummy_api_client();
    let mut user = User::default();
    user.first_name = "Father".to_string();
    user.last_name = "Dougal".to_string();
    user.email = "random_email@urrland.com".to_string();
    user.date_of_birth = "1/1/1900".to_string();
    for _ in 0..101 {
        let start = time::Instant::now();
        let user = client
            .post("data/v1/user/create")
            .body(&user)
            .expect_json::<User>()
            .unwrap();
        let route = format!("data/v1/user/{}", user.id);
        let result = client.delete(&route).send().unwrap();
        let end = time::Instant::now().duration_since(start);
        assert!(result.is_ok());
        results.push(end);
    }
    print_results(results);
}
