use actix_web::cookie::Cookie;
use actix_web::http::header::ContentType;
use actix_web::{get, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use askama::Template;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    error_message: &'a str,
}

#[get("/login")]
pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let error = match flash_messages.iter().find(|m| m.level() == Level::Error) {
        Some(x) => x.content(),
        None => "",
    };
    let login_form = LoginTemplate {
        error_message: error,
    };

    let mut response = HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(login_form.render().unwrap());
    response
        .add_removal_cookie(&Cookie::new("_flash", ""))
        .unwrap();
    response
}
