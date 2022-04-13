use crate::session_state::TypedSession;
use actix_web::http::header::LOCATION;
use actix_web::{get, HttpResponse};

#[get("/admin/logout")]
#[tracing::instrument(skip(session))]
pub async fn logout_user(session: TypedSession) -> HttpResponse {
    session.purge();
    HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish()
}
