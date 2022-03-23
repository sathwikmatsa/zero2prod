use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use actix_web::{post, web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use sqlx::types::uuid;
use sqlx::PgPool;
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
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscription(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let subscriber = match form.0.try_into() {
        Ok(form) => form,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match insert_subscriber(&pool, &subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(subscriber, pool)
)]
async fn insert_subscriber(pool: &PgPool, subscriber: &NewSubscriber) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        subscriber.email.as_ref(),
        subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
