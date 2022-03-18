use std::net::TcpListener;

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{address}/health_check"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}

fn spawn_app() -> String {
    let tcp_listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");
    let port = tcp_listener.local_addr().unwrap().port();
    let server = zero2prod::run(tcp_listener).expect("Failed to use listener");
    let _ = tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}
