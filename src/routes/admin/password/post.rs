use crate::authentication::{update_password_hash, validate_credentials, Credentials};
use crate::session_state::TypedSession;
use crate::util::{e500, get_username, see_other};
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    confirm_new_password: Secret<String>,
}

#[tracing::instrument(
    skip(form, pool, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
#[post("/admin/password")]
pub async fn change_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
        get_username(user_id, &pool).await.map_err(e500)?
    } else {
        return Ok(see_other("/login"));
    };

    if form.0.new_password.expose_secret() != form.0.confirm_new_password.expose_secret() {
        FlashMessage::error("New password does not match with confirmation password.").send();
        return Ok(see_other("/admin/password"));
    }

    if !matches!(form.0.new_password.expose_secret().len(), 12..=127) {
        FlashMessage::error(
            "New password must at least 12 characters long but shorter than 128 characters.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };

    match validate_credentials(credentials.clone(), &pool).await {
        Ok(_user_id) => {
            update_password_hash(credentials, &pool)
                .await
                .map_err(e500)?;
            session.purge();
            FlashMessage::info("Password updated successfully. Please login to continue.").send();
            Ok(see_other("/login"))
        }
        Err(_e) => {
            FlashMessage::error("Current password is incorrect.").send();
            Ok(see_other("/admin/password"))
        }
    }
}
