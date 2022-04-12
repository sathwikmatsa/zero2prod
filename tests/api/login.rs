use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    // 1 - try login
    let response = app.post_login(&login_body).await;
    assert_eq!(response.status().as_u16(), 303);
    assert_is_redirect_to(&response, "/login");

    // 2 - follow redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    // 3 - reload page
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let app = spawn_app().await;

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
    "username": &app.test_user.username,
    "password": &app.test_user.password
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));
}
