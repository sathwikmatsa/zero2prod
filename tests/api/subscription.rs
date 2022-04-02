use crate::helper::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = "name=sathwik%20matsa&email=sathwikmatsa%40gmail.com";
    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_an_error_response_for_invalid_form_data() {
    let app = spawn_app().await;
    let (invalid_name, valid_name) = ("sat,wik", "sathwik");
    let (invalid_email, valid_email) = ("abc.com", "abc@xyz.com");

    let test_cases = vec![
        (
            valid_name,
            invalid_email,
            format!("{} is not a valid subscriber email.", invalid_email),
        ),
        (
            invalid_name,
            valid_email,
            format!("{} is not a valid subscriber name.", invalid_name),
        ),
    ];

    for (name, email, error_response) in test_cases {
        let body = format!("name={}&email={}", name, email);

        let response = app.post_subscriptions(body.into()).await;
        let body = response.text().await.unwrap();

        assert_eq!(error_response, body);
    }
}

#[tokio::test]
async fn subscribe_returns_an_error_response_when_subscription_is_already_confirmed() {
    let app = spawn_app().await;
    let body = "name=sathwik%20matsa&email=sathwikmatsa%40gmail.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);
    reqwest::get(confirmation_links.html).await.unwrap();

    let response = app.post_subscriptions(body.into()).await;
    let body = response.text().await.unwrap();

    assert_eq!(
        "Failed to subscribe as the subscriber is already confirmed.",
        body
    );
}

#[tokio::test]
async fn subscribe_persists_new_subscriber() {
    let app = spawn_app().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = "name=sathwik%20matsa&email=sathwikmatsa%40gmail.com";
    let _response = app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "sathwikmatsa@gmail.com");
    assert_eq!(saved.name, "sathwik matsa");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;

    let test_cases = vec![
        ("name=sathwik%20matsa", "missing the email"),
        ("email=sathwikmatsa%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    let app = spawn_app().await;

    let test_cases = vec![
        ("name=sathwik%20matsa&email=", "empty email"),
        ("email=sathwikmatsa%40gmail.com&name=", "empty name"),
        ("name=&email=", "empty name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = "name=sathwik%20matsa&email=sathwikmatsa%40gmail.com";
    let _response = app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);
    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn new_confirmation_email_is_sent_when_user_subscribes_again() {
    let app = spawn_app().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&app.email_server)
        .await;

    let body = "name=sathwik%20matsa&email=sathwikmatsa%40gmail.com";
    let _response = app.post_subscriptions(body.into()).await;
    let _first_email_request = &app.email_server.received_requests().await.unwrap()[0];

    // user tries to subscribe again, ignoring previous confirmation email
    let _response = app.post_subscriptions(body.into()).await;
    let _second_email_request = &app.email_server.received_requests().await.unwrap()[1];

    // Assert new email is sent to the user.
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
        .execute(&app.db_pool)
        .await
        .unwrap();
    // Act
    let response = app.post_subscriptions(body.into()).await;
    // Assert
    assert_eq!(response.status().as_u16(), 500);
}
