use crate::domain::subscription_token::SubscriptionToken;
use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use actix_web::{post, web, HttpResponse};
use chrono::Utc;
use reqwest::Url;
use serde::Deserialize;
use sqlx::types::uuid;
use sqlx::{PgPool, Postgres, Transaction};
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
) -> HttpResponse {
    let subscriber = match form.0.try_into() {
        Ok(form) => form,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let subscriber_status = match insert_or_get_subscriber(&mut transaction, &subscriber).await {
        Ok(subscriber_status) => subscriber_status,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if subscriber_status.confirmed() {
        return HttpResponse::BadRequest().finish();
    }

    let subscription_token = SubscriptionToken::new();
    // Note: This could result in multiple subscription tokens for the same subscriber id.
    // This is caused when subscriber (pending_confirmation) tries to subscribe again
    // New record with new token is stored in the table when this function is called.
    // User behaviour is not impacted i.e., new confirmation link is sent out on retry.
    // But there could be multiple stale entries for the same subscriber even after confirmation.
    // We could add TTL to the records to avoid stagnation. (TODO)
    if store_token(&mut transaction, subscriber_status.id, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(&email_client, subscriber, &base_url.0, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
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
    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(status)
}
