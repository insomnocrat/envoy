use crate::rest::{Client, ClientConfig};

fn dummy_api_client() -> Client {
    ClientConfig::from("dummyapi.io")
        .required_header(("app-id", "623e3f74a76d8facdad7758b"))
        .into()
}

#[test]
fn read_response() {
    let mut client = dummy_api_client();
    let user_list = client.get("data/v1/user").expect_model().unwrap();
    let result = client.interpret(user_list, Some("UserList"));
    assert!(result.is_ok());
}
#[test]
fn read_response_as_optional() {
    let mut client = dummy_api_client();
    client.all_optional_fields();
    let user_list = client.get("data/v1/user").expect_model().unwrap();
    let result = client.interpret(user_list, Some("UserList"));
    assert!(result.is_ok());
}
#[test]
fn read_response_as_default() {
    let mut client = dummy_api_client();
    client.no_optional_fields();
    let user_list = client.get("data/v1/user").expect_model().unwrap();
    let result = client.interpret(user_list, Some("UserList"));
    assert!(result.is_ok());
}
#[test]
fn read_response_indvidual_files() {
    let mut client = dummy_api_client();
    client.individual_files();
    let user_list = client.get("data/v1/user").expect_model().unwrap();
    let result = client.interpret(user_list, Some("UserList"));
    assert!(result.is_ok());
}
#[test]
fn read_response_mod_folder() {
    let mut client = dummy_api_client();
    client.model_folder("models");
    let user_list = client.get("data/v1/user").expect_model().unwrap();
    let result = client.interpret(user_list, Some("UserList"));
    assert!(result.is_ok());
}
