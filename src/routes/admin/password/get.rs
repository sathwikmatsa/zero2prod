use crate::authentication::UserId;
use crate::util::{e500, get_username};
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

#[get("/password")]
pub async fn change_password_form(
    flash_messages: IncomingFlashMessages,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &pool).await.map_err(e500)?;

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
