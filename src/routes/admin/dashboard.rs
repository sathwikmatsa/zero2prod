use crate::session_state::TypedSession;
use crate::util::{e500, get_username, see_other};
use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse};
use askama::Template;
use sqlx::PgPool;

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
        return Ok(see_other("/login"));
    };

    let admin_dashboard = DashboardTemplate {
        username: username.as_str(),
    };
    let admin_dashboard_html = admin_dashboard.render().map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(admin_dashboard_html))
}
