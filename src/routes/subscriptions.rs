use actix_web::{post, web, HttpResponse};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

#[post("/subscriptions")]
pub async fn subscription(form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
