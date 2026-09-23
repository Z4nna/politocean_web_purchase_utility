use crate::data::errors::DataError;
use dotenvy::dotenv;
use lettre::{
    message::header, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};

/// Sends a simple plain-text email through the configured Gmail SMTP relay.
/// Credentials are read from the SMTP_USER / SMTP_PASS environment variables.
pub async fn send_plaintext_email(to: &str, subject: &str, body: String) -> Result<(), DataError> {
    dotenv().ok();
    let smtp_user = std::env::var("SMTP_USER").map_err(|e| DataError::Mail(e.to_string()))?;
    let smtp_pass = std::env::var("SMTP_PASS").map_err(|e| DataError::Mail(e.to_string()))?;

    let email = Message::builder()
        .from(
            smtp_user
                .parse()
                .map_err(|e: lettre::address::AddressError| DataError::Mail(e.to_string()))?,
        )
        .to(to
            .parse()
            .map_err(|e: lettre::address::AddressError| DataError::Mail(e.to_string()))?)
        .subject(subject)
        .header(header::ContentType::TEXT_PLAIN)
        .body(body)
        .map_err(|e| DataError::Mail(e.to_string()))?;

    let creds = Credentials::new(smtp_user, smtp_pass);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay("smtp.gmail.com")
        .map_err(|e| DataError::Mail(e.to_string()))?
        .credentials(creds)
        .build();

    mailer
        .send(email)
        .await
        .map_err(|e| DataError::Mail(e.to_string()))?;
    Ok(())
}
