use crate::helper::{assert_is_redirect_to, spawn_app};
use uuid::Uuid;

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_change_password_form() {
    let app = spawn_app().await;
    let response = app.get_change_password_form().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_password() {
    let app = spawn_app().await;

    let change_pass_body = serde_json::json!({
        "current_password": "random-password-1",
        "new_password": "random-password-2",
        "confirm_new_password": "random-password-2"
    });

    let response = app.post_change_pass(&change_pass_body).await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn an_error_flash_message_is_set_on_invalid_password() {
    let app = spawn_app().await;

    // 1 - login
    app.login().await;

    let change_pass_body = serde_json::json!({
        "current_password": "random-password-1",
        "new_password": "random-password-2",
        "confirm_new_password": "random-password-2"
    });

    // 2 - post change password
    let response = app.post_change_pass(&change_pass_body).await;
    assert_is_redirect_to(&response, "/admin/password");

    // 3 - follow redirect
    let html_page = app.get_change_password_form_html().await;
    assert!(html_page.contains(r#"<p><i>Current password is incorrect.</i></p>"#));
}

#[tokio::test]
async fn an_error_flash_message_is_set_on_unmatched_confirm_new_password() {
    let app = spawn_app().await;

    // 1 - login
    app.login().await;

    let change_pass_body = serde_json::json!({
        "current_password": "random-password-1",
        "new_password": "random-password-2",
        "confirm_new_password": "random-password-3"
    });

    // 2 - post change password
    let response = app.post_change_pass(&change_pass_body).await;
    assert_is_redirect_to(&response, "/admin/password");

    // 3 - follow redirect
    let html_page = app.get_change_password_form_html().await;
    assert!(html_page
        .contains(r#"<p><i>New password does not match with confirmation password.</i></p>"#));
}

#[tokio::test]
async fn an_error_flash_message_is_set_on_invalid_new_password() {
    let app = spawn_app().await;

    // 1 - login
    app.login().await;

    let change_pass_body = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": "short",
        "confirm_new_password": "short"
    });

    // 2 - post change password
    let response = app.post_change_pass(&change_pass_body).await;
    assert_is_redirect_to(&response, "/admin/password");

    // 3 - follow redirect
    let html_page = app.get_change_password_form_html().await;
    assert!(html_page.contains(r#"<p><i>New password must at least 12 characters long but shorter than 128 characters.</i></p>"#));
}

#[tokio::test]
async fn redirect_to_login_after_successful_pass_change() {
    let app = spawn_app().await;

    // 1 - login
    app.login().await;

    let new_password = Uuid::new_v4().to_string();
    let change_pass_body = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": new_password,
        "confirm_new_password": new_password,
    });

    // 2 - post change password
    let response = app.post_change_pass(&change_pass_body).await;
    assert_is_redirect_to(&response, "/login");

    // 3 - follow redirect
    let html_page = app.get_login_html().await;
    assert!(html_page
        .contains(r#"<p><i>Password updated successfully. Please login to continue.</i></p>"#));

    // 4 - try to open admin dashboard
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
