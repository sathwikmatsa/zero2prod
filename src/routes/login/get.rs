use actix_web::cookie::Cookie;
use actix_web::http::header::ContentType;
use actix_web::{get, HttpRequest, HttpResponse};
use askama::Template;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    error_message: &'a str,
}

#[get("/login")]
pub async fn login_form(request: HttpRequest) -> HttpResponse {
    let error = request
        .cookie("_flash")
        .map(|c| c.value().to_owned())
        .unwrap_or_else(|| "".into());
    let login_form = LoginTemplate {
        error_message: error.as_str(),
    };

    let mut response = HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(login_form.render().unwrap());
    response
        .add_removal_cookie(&Cookie::new("_flash", ""))
        .unwrap();
    response
}
