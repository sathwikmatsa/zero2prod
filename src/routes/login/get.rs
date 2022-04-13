use crate::util::e500;
use actix_web::http::header::ContentType;
use actix_web::{get, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use askama::Template;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    messages: Vec<&'a str>,
}

#[get("/login")]
pub async fn login_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let messages = flash_messages
        .iter()
        .map(|m| m.content())
        .collect::<Vec<_>>();
    let login_form = LoginTemplate { messages };
    let login_form_html = login_form.render().map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(login_form_html))
}
