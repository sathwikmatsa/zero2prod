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

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });

    let response = app.post_newsletter(&newsletter_request_body).await;
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

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });

    app.post_newsletter(&newsletter_request_body).await;
    let html_page = app.get_newsletter_form_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
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

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });

    let response = app.post_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletter");

    let html = app.get_newsletter_form_html().await;
    assert!(html.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[tokio::test]
async fn newsletter_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    app.login().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "text_content": "Newsletter body as plain text",
                "html_content": "<p>Newsletter body as HTML</p>",
            }),
            "Parse error: missing field `title`.",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "text_content": "Newsletter body as plain text",
            }),
            "Parse error: missing field `html_content`.",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "html_content": "<p>Newsletter body as HTML</p>",
            }),
            "Parse error: missing field `text_content`.",
        ),
        (
            serde_json::json!({
                "title": "",
                "text_content": "Newsletter body as plain text",
                "html_content": "<p>Newsletter body as HTML</p>",
            }),
            "Validation error: field `title` cannot be empty.",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "text_content": "Newsletter body as plain text",
                "html_content": "",
            }),
            "Validation error: field `html_content` cannot be empty.",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "html_content": "<p>Newsletter body as HTML</p>",
                "text_content": "",
            }),
            "Validation error: field `text_content` cannot be empty.",
        ),
    ];
    for (invalid_body, flash_message) in test_cases {
        let response = app.post_newsletter(&invalid_body).await;
        assert_is_redirect_to(&response, "/admin/newsletter");

        let html = app.get_newsletter_form_html().await;

        assert!(html.contains(flash_message));
    }
}
