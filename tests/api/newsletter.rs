use crate::helper::{assert_is_redirect_to, spawn_app};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn you_must_be_logged_in_to_access_newsletter_form() {
    let app = spawn_app().await;
    let response = app.get_newsletter_form().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_post_newsletter() {
    let app = spawn_app().await;

    let body = "title=Newsletter%20title&\
    html_content=%3Cp%3ENewsletter%20body%20as%20HTML%3C%2Fp%3E&\
    text_content=Newsletter%20body%20as%20plain%20text";

    let response = app.post_newsletters(body.into()).await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    app.login().await;

    app.create_unconfirmed_subscriber().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request is fired at Postmark!
        .expect(0)
        .mount(&app.email_server)
        .await;

    let body = "title=Newsletter%20title&\
    html_content=%3Cp%3ENewsletter%20body%20as%20HTML%3C%2Fp%3E&\
    text_content=Newsletter%20body%20as%20plain%20text";

    let response = app.post_newsletters(body.into()).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    app.login().await;

    app.create_confirmed_subscriber().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let body = "title=Newsletter%20title&\
    html_content=%3Cp%3ENewsletter%20body%20as%20HTML%3C%2Fp%3E&\
    text_content=Newsletter%20body%20as%20plain%20text";

    let response = app.post_newsletters(body.into()).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletter_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    app.login().await;
    let test_cases = vec![
        (
            "html_content=%3Cp%3ENewsletter%20body%20as%20HTML%3C%2Fp%3E&\
            text_content=Newsletter%20body%20as%20plain%20text",
            "missing title",
        ),
        (
            "title=Newsletter%20title&\
            text_content=Newsletter%20body%20as%20plain%20text",
            "missing html content",
        ),
        (
            "title=Newsletter%20title&\
            html_content=%3Cp%3ENewsletter%20body%20as%20HTML%3C%2Fp%3E",
            "missing text content",
        ),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
