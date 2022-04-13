use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn logged_out_user_cannot_access_admin_dashboard() {
    let app = spawn_app().await;

    // Act 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act 2 - Follow the redirect
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    // Act 3 - Logout
    let response = app.logout().await;
    assert_is_redirect_to(&response, "/login");

    // Act 4 - Try to open admin dashboard
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
