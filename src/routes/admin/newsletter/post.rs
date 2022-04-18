use crate::authentication::UserId;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::idempotency::{get_saved_response, save_response, IdempotencyKey};
use crate::util::{e400, e500, see_other};
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

// TODO: replace with a custom non empty string type
impl FormData {
    fn validate(&self) -> Result<(), String> {
        if self.title.is_empty() {
            Err("Validation error: field `title` cannot be empty.".into())
        } else if self.text_content.is_empty() {
            Err("Validation error: field `text_content` cannot be empty.".into())
        } else if self.html_content.is_empty() {
            Err("Validation error: field `html_content` cannot be empty.".into())
        } else {
            Ok(())
        }
    }
}

#[post("/newsletter")]
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(form, pool, email_client),
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    form: Result<web::Form<FormData>, actix_web::Error>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    // TODO: no error trace in logs, wrap with BadRequest?
    let form = match form {
        Ok(form) => match form.validate() {
            Ok(_) => form,
            Err(e) => return Ok(send_flash_message_and_redirect(e, "/admin/newsletter")),
        },
        Err(e) => return Ok(send_flash_message_and_redirect(e, "/admin/newsletter")),
    };
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    let user_id = user_id.into_inner();
    if let Some(saved_response) = get_saved_response(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        FlashMessage::info("The newsletter issue has been published!").send();
        return Ok(saved_response);
    }

    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(&subscriber.email, &title, &html_content, &text_content)
                .await
                .with_context(|| format!("Failed to send newsletter issue to {}", subscriber.email))
                .map_err(e500)?,
            Err(error) => {
                tracing::warn!(
                error.cause_chain = ?error,
                "Skipping a confirmed subscriber. \
                Their stored contact details are invalid",
                );
            }
        }
    }
    FlashMessage::info("The newsletter issue has been published!").send();
    let response = see_other("/admin/newsletter");
    let response = save_response(&pool, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();
    Ok(confirmed_subscribers)
}

fn send_flash_message_and_redirect(error: impl ToString, location: &str) -> HttpResponse {
    FlashMessage::error(error.to_string()).send();
    see_other(location)
}
