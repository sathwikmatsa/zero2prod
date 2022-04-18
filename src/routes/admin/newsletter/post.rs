use crate::authentication::UserId;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::idempotency::{get_saved_response, save_response, IdempotencyKey};
use crate::util::{e500, see_other, NonEmptyString};
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: NonEmptyString,
    text_content: NonEmptyString,
    html_content: NonEmptyString,
    idempotency_key: IdempotencyKey,
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
    let form = match form {
        Ok(f) => f,
        Err(e) => {
            return Ok(send_flash_message_and_redirect(e, "/admin/newsletter"));
        }
    };
    let user_id = user_id.into_inner();
    if let Some(saved_response) = get_saved_response(&pool, &form.0.idempotency_key, *user_id)
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
                .send_email(
                    &subscriber.email,
                    form.0.title.as_ref(),
                    form.0.html_content.as_ref(),
                    form.0.text_content.as_ref(),
                )
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
    let response = save_response(&pool, &form.0.idempotency_key, *user_id, response)
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
