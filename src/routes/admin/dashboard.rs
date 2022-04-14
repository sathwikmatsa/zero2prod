use crate::authentication::UserId;
use crate::util::{e500, get_username};
use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse};
use askama::Template;
use sqlx::PgPool;

#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct DashboardTemplate<'a> {
    username: &'a str,
}

#[get("/dashboard")]
#[tracing::instrument(skip(pool, user_id), fields(user_id=%*user_id))]
pub async fn admin_dashboard(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &pool).await.map_err(e500)?;

    let admin_dashboard = DashboardTemplate {
        username: username.as_str(),
    };
    let admin_dashboard_html = admin_dashboard.render().map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(admin_dashboard_html))
}
