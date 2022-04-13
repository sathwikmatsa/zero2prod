use crate::session_state::TypedSession;
use crate::util::{e500, get_username, see_other};
use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use askama::Template;
use sqlx::PgPool;

#[derive(Template)]
#[template(path = "change_password.html")]
struct PasswordFormTemplate<'a> {
    username: &'a str,
    messages: Vec<&'a str>,
}

#[get("/admin/password")]
#[tracing::instrument(
skip(session, pool, flash_messages),
fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn change_password_form(
    flash_messages: IncomingFlashMessages,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
        get_username(user_id, &pool).await.map_err(e500)?
    } else {
        return Ok(see_other("/login"));
    };
    tracing::Span::current().record("username", &tracing::field::display(&username));

    let messages = flash_messages
        .iter()
        .map(|m| m.content())
        .collect::<Vec<_>>();

    let password_form = PasswordFormTemplate {
        messages,
        username: username.as_str(),
    };
    let password_form_html = password_form.render().map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(password_form_html))
}
