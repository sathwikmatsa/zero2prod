pub mod authentication;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod routes;
pub mod session_state;
pub mod startup;
pub mod telemetry;

pub fn error_chain_fmt(
    err: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", err)?;
    let mut current = err.source();
    while let Some(cause) = current {
        write!(f, "Caused by:\n\t{}\n", cause)?;
        current = cause.source();
    }
    Ok(())
}
