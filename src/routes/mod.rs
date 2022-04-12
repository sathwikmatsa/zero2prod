mod admin;
mod health_check;
mod home;
mod login;
mod newsletter;
mod subscription_confirm;
mod subscriptions;

pub use admin::*;
pub use health_check::*;
pub use home::*;
pub use login::*;
pub use newsletter::*;
pub use subscription_confirm::*;
pub use subscriptions::*;

pub fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}
