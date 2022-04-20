use crate::authentication::UserId;
use crate::idempotency::{save_response, try_processing, IdempotencyKey, NextAction};
use crate::util::{e500, see_other, NonEmptyString};
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

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
    skip(form, pool),
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    form: Result<web::Form<FormData>, actix_web::Error>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let form = match form {
        Ok(f) => f,
        Err(e) => {
            return Ok(send_flash_message_and_redirect(e, "/admin/newsletter"));
        }
    };
    let user_id = user_id.into_inner();
    let mut transaction = match try_processing(&pool, &form.0.idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };
    let issue_id = insert_newsletter_issue(
        &mut transaction,
        form.0.title.as_ref(),
        form.0.text_content.as_ref(),
        form.0.html_content.as_ref(),
    )
    .await
    .context("Failed to store newsletter issue details")
    .map_err(e500)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;
    success_message().send();
    let response = see_other("/admin/newsletter");
    let response = save_response(transaction, &form.0.idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    )
    .execute(transaction)
    .await?;
    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(transaction)
    .await?;
    Ok(())
}

fn send_flash_message_and_redirect(error: impl ToString, location: &str) -> HttpResponse {
    FlashMessage::error(error.to_string()).send();
    see_other(location)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been accepted - emails will go out shortly.")
}
