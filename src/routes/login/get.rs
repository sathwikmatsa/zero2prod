use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse};
use askama::Template;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    error_message: &'a str,
}

#[get("/login")]
pub async fn login_form(query: web::Query<QueryParams>) -> HttpResponse {
    let error = query.error.as_deref().unwrap_or("");
    let login_form = LoginTemplate {
        error_message: error,
    };
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(login_form.render().unwrap())
}
