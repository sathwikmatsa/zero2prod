use crate::startup::HmacSecret;
use actix_web::http::header::ContentType;
use actix_web::{get, web, HttpResponse};
use askama::Template;
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verify(&self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag.as_str())?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;
        Ok(self.error.clone())
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    error_message: &'a str,
}

#[get("/login")]
pub async fn login_form(
    query: Option<web::Query<QueryParams>>,
    secret: web::Data<HmacSecret>,
) -> HttpResponse {
    let error = query
        .map(|p| {
            p.verify(&secret).unwrap_or_else(|e| {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to verify query parameters using the HMAC tag"
                );
                "".into()
            })
        })
        .unwrap_or_else(|| "".into());

    let login_form = LoginTemplate {
        error_message: error.as_str(),
    };
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(login_form.render().unwrap())
}
