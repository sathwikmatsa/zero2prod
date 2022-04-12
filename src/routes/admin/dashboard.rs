use crate::routes::e500;
use crate::session_state::TypedSession;
use actix_web::http::header::{ContentType, LOCATION};
use actix_web::{get, web, HttpResponse};
use anyhow::Context;
use askama::Template;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct DashboardTemplate<'a> {
    username: &'a str,
}

#[get("/admin/dashboard")]
#[tracing::instrument(skip(session, pool))]
pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        get_username(user_id, &pool).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };

    let admin_dashboard = DashboardTemplate {
        username: username.as_str(),
    };
    let admin_dashboard_html = admin_dashboard.render().map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(admin_dashboard_html))
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.username)
}
