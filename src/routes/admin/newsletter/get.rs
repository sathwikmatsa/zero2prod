use crate::authentication::UserId;
use crate::util::{e500, get_username};
use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use askama::Template;
use sqlx::PgPool;

#[derive(Template)]
#[template(path = "newsletter_form.html")]
struct NewsletterFormTemplate<'a> {
    messages: Vec<&'a str>,
}

#[get("/newsletter")]
#[tracing::instrument(
    skip(pool, flash_messages),
    fields(username=tracing::field::Empty)
)]
pub async fn newsletter_form(
    flash_messages: IncomingFlashMessages,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &pool).await.map_err(e500)?;
    tracing::Span::current().record("username", &tracing::field::display(&username));

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
