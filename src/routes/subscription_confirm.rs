use crate::domain::subscription_token::SubscriptionToken;
use crate::util::error_chain_fmt;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use std::fmt;
use std::fmt::Formatter;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error)]
pub enum SubscriptionConfirmError {
    #[error("{0}")]
    Validation(String),
    #[error("Failed to confirm subscription because of unauthorized token.")]
    UnauthorizedToken,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl fmt::Debug for SubscriptionConfirmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        error_chain_fmt(&self, f)
    }
}

impl ResponseError for SubscriptionConfirmError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::UnauthorizedToken => StatusCode::UNAUTHORIZED,
        }
    }
}

#[get("/subscriptions/confirm")]
#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, SubscriptionConfirmError> {
    let subscription_token = SubscriptionToken::parse(&parameters.subscription_token)
        .map_err(SubscriptionConfirmError::Validation)?;

    let id = get_subscriber_id_from_token(&pool, &subscription_token)
        .await
        .context("Failed to get subscriber id from the database.")?;
    match id {
        // Non-existing token!
        None => Err(SubscriptionConfirmError::UnauthorizedToken),
        Some(subscriber_id) => {
            confirm_subscriber(&pool, subscriber_id)
                .await
                .context("Failed to update subscriber as confirmed in the database.")?;
            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &SubscriptionToken,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token.as_ref(),
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}
