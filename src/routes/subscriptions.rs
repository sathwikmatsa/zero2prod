use crate::domain::subscription_token::SubscriptionToken;
use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use crate::{error_chain_fmt, TEMPLATES};
use actix_web::body::BoxBody;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, ResponseError};
use anyhow::Context;
use chrono::Utc;
use reqwest::Url;
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use std::fmt;
use std::fmt::Formatter;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let email = SubscriberEmail::parse(value.email)?;
        let name = SubscriberName::parse(value.name)?;
        Ok(Self { email, name })
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Failed to subscribe as the subscriber is already confirmed.")]
    AlreadyConfirmedError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        error_chain_fmt(&self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ValidationError(_) | Self::AlreadyConfirmedError => StatusCode::BAD_REQUEST,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            Self::ValidationError(_) | Self::AlreadyConfirmedError => {
                HttpResponse::build(self.status_code())
                    .content_type(ContentType::plaintext())
                    .body(self.to_string())
            }
            _ => HttpResponse::new(self.status_code()),
        }
    }
}

#[post("/subscriptions")]
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscription(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from the pool.")?;

    let subscriber_status = insert_or_get_subscriber(&mut transaction, &subscriber)
        .await
        .context("Failed to insert new subscriber in the database.")?;

    if subscriber_status.confirmed() {
        return Err(SubscribeError::AlreadyConfirmedError);
    }

    let subscription_token = SubscriptionToken::new();
    // Note: This could result in multiple subscription tokens for the same subscriber id.
    // This is caused when subscriber (pending_confirmation) tries to subscribe again
    // New record with new token is stored in the table when this function is called.
    // User behaviour is not impacted i.e., new confirmation link is sent out on retry.
    // But there could be multiple stale entries for the same subscriber even after confirmation.
    // We could add TTL to the records to avoid stagnation. (TODO)
    store_token(&mut transaction, subscriber_status.id, &subscription_token)
        .await
        .context("Failed to store subscription token in the database")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;

    send_confirmation_email(&email_client, subscriber, &base_url.0, &subscription_token)
        .await
        .context("Failed to send a confirmation email.")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &SubscriptionToken,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token.as_ref(),
        subscriber_id
    )
    .execute(transaction)
    .await?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, subscriber, base_url, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: NewSubscriber,
    base_url: &Url,
    subscription_token: &SubscriptionToken,
) -> Result<(), reqwest::Error> {
    let confirmation_link = base_url
        .join(&format!(
            "subscriptions/confirm?subscription_token={}",
            subscription_token.as_ref()
        ))
        .unwrap();
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    let mut context = tera::Context::new();
    context.insert("confirmation_link", confirmation_link.as_ref());

    let html_body = match TEMPLATES.render("email.template", &context) {
        Ok(content) => content,
        Err(_) => plain_body.clone(),
    };

    email_client
        .send_email(subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

struct SubscriberStatus {
    id: Uuid,
    status: String,
}

impl SubscriberStatus {
    fn confirmed(&self) -> bool {
        self.status == "confirmed"
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(subscriber, transaction)
)]
async fn insert_or_get_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber: &NewSubscriber,
) -> Result<SubscriberStatus, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let status = sqlx::query_as!(
        SubscriberStatus,
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name
        RETURNING id, status;
        "#,
        subscriber_id,
        subscriber.email.as_ref(),
        subscriber.name.as_ref(),
        Utc::now()
    )
    .fetch_one(transaction)
    .await?;
    Ok(status)
}
