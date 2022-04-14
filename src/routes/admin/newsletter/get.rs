use crate::util::e500;
use actix_web::http::header::ContentType;
use actix_web::{get, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use askama::Template;

#[derive(Template)]
#[template(path = "newsletter_form.html")]
struct NewsletterFormTemplate<'a> {
    messages: Vec<&'a str>,
}

#[get("/newsletter")]
pub async fn newsletter_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let messages = flash_messages
        .iter()
        .map(|m| m.content())
        .collect::<Vec<_>>();

    let newsletter_form = NewsletterFormTemplate { messages };
    let newsletter_form_html = newsletter_form.render().map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(newsletter_form_html))
}
