use std::{time::Duration, usize};
use askama::Template;
use dotenvy::dotenv;
use lettre::{message::{header, MultiPart, SinglePart}, transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use crate::{
    data::{errors::{self, AppError, DataError}, order, user}, handlers::{self, edit_order}, models::{app::AppState, templates::ProfHomepageTemplate}
};
use axum::{
    extract::{Path, State}, response::{Html, IntoResponse, Redirect, Response}, Json
};
use tower_sessions::Session;

pub async fn prof_homepage_handler(
    State(app_state): State<AppState>,
    session: Session,
) -> Result<Response, errors::AppError> {
    // check if the user is authenticated
    let user_id = session.get::<i32>("authenticated_user_id")
    .await
    .map_err(|e| errors::AppError::Session(e))?;
    match user_id {
        Some(id) => {
            // if user is logged in, check if he got enough permissions
            let user_role_result = user::get_user_role(&app_state.connection_pool, id).await;
            if let Ok(user_role) = user_role_result {
                if user_role != "prof" {
                    return Ok(Redirect::to("/home").into_response());
                } else {
                    let html_string = ProfHomepageTemplate {
                        orders: order::get_confirmed_orders(&app_state.connection_pool).await?,
                    }.render().unwrap();
                    return Ok(Html(html_string).into_response());
                }
            }
            Ok(Redirect::to("/home").into_response())
        }
        None => {
            // If user is not logged in, redirect to login page
            Ok(Redirect::to("/").into_response())
        }
    }
}


#[derive(Serialize, Deserialize)]
pub struct OrderNotificationRequest {
    pub order_id: i32,
    pub user_id: i32,
    // pub datetime

}

pub async fn notify_prof_order_confirmed_handler(
    State(app_state): State<AppState>,
    session: Session,
    Json(payload): Json<OrderNotificationRequest>
) -> Result<(), AppError> {
    // generate bom
    handlers::edit_order::generate_bom_handler(State(app_state.clone()), session.clone(), Path(payload.order_id)).await?;
    
    wait_for_bom_job_to_finish(payload.order_id, app_state.clone()).await?;

    // download carts
    let mouser_cart = axum::body::to_bytes(
        handlers::edit_order::download_mouser_cart_handler(
            State(app_state.clone()),
            session.clone(),
            Path(payload.order_id))
            .await?
            .into_body(), 
        usize::MAX)
        .await.map_err(|e| DataError::Internal(e.to_string()))?;
    let digikey_cart = axum::body::to_bytes(
        handlers::edit_order::download_digikey_cart_handler(
            State(app_state.clone()),
            session.clone(),
            Path(payload.order_id))
            .await?
            .into_body(), 
        usize::MAX)
        .await.map_err(|e| DataError::Internal(e.to_string()))?;

    // download bom
    let bom_data = sqlx::query!(
        "SELECT filename, bom_file_mouser, bom_file_digikey FROM order_bom WHERE order_id = $1",
        payload.order_id
    ).fetch_one(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::FailedQuery(e.to_string()))?;
    println!("constructing email...");
    // email construction
    let subject = format!("PoliTOcean: conferma ordine #{}", payload.order_id);
    let recipient = std::env::var("ORDER_NOTIFICATION_RECIPIENT_EMAIL_ADDR").expect("Recipient email not found");
    let order_data = sqlx::query!(
        "SELECT author_id, description FROM orders WHERE id = $1",
        payload.order_id
    ).fetch_one(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::FailedQuery(e.to_string()))?;
    let author_data = sqlx::query!(
        "SELECT username, email FROM users WHERE id = $1",
        order_data.author_id
    ).fetch_one(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::FailedQuery(e.to_string()))?;
    let board_member_data = sqlx::query!(
        "SELECT username, email FROM users WHERE id = $1",
        payload.user_id
    ).fetch_one(&app_state.connection_pool)
    .await
    .map_err(|e| DataError::FailedQuery(e.to_string()))?;
    let mail_body_text = format!(
        "
            Buongiorno professore,\n\n
            un ordine è stato confermato da {} (id: {}, mail: {}) in data odierna.\n\n
            Ordine #{}: {}\n
            Autore: {} (id: {}, mail: {})\n\n
            Allegati a questa mail troverà i file di preventivo separati per mouser e digikey (BOM),\n
            insieme a due altri file (aventi \"cart\" nel nome) che le permetteranno di aggiungere automaticamente gli oggetti al carrello.\n\n
            Le auguriamo una buona giornata,\n
            Team PoliTOcean.
        ",
        board_member_data.username,
        payload.user_id,
        board_member_data.email.unwrap_or("not found".to_string()),
        payload.order_id, 
        order_data.description,
        author_data.username,
        order_data.author_id,
        author_data.email.unwrap_or("not found".to_string()),
    ); 

    let xlsx_ct: header::ContentType = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        .parse()
        .map_err(|e: header::ContentTypeErr| DataError::Mail(e.to_string()))?;
    let text_ct: header::ContentType = "text/plain; charset=utf-8"
        .parse()
        .map_err(|e: header::ContentTypeErr| DataError::Mail(e.to_string()))?;

    let mail_body = SinglePart::builder()
            .header(text_ct.clone())
            .body(mail_body_text);

    if mouser_cart.is_empty() 
        || digikey_cart.is_empty() 
        || bom_data.bom_file_mouser.is_none()
        || bom_data.bom_file_mouser.clone().unwrap_or_default().is_empty()
        || bom_data.bom_file_digikey.is_none() 
        || bom_data.bom_file_digikey.clone().unwrap_or_default().is_empty() {
            return Err(AppError::Database(DataError::Internal("Error generating BOM files or carts.".to_string())));
    }

    let mouser_bom_att = SinglePart::builder()
        .header(xlsx_ct.clone())
        .header(header::ContentDisposition::attachment(&format!("{}_mouser_{}.xlsx", bom_data.filename.clone().unwrap_or("name not found".to_string()), payload.order_id)))
        .body(bom_data.bom_file_mouser.unwrap_or_default());
    let digikey_bom_att = SinglePart::builder()
        .header(xlsx_ct.clone())
        .header(header::ContentDisposition::attachment(&format!("{}_digikey_{}.xlsx", bom_data.filename.clone().unwrap_or("name not found".to_string()), payload.order_id)))
        .body(bom_data.bom_file_digikey.unwrap_or_default());
    let mouser_cart_att = SinglePart::builder()
        .header(xlsx_ct.clone())
        .header(header::ContentDisposition::attachment(&format!("cart_mouser_{}.xlsx", payload.order_id)))
        .body(mouser_cart.to_vec());
    let digikey_cart_att = SinglePart::builder()
        .header(xlsx_ct)
        .header(header::ContentDisposition::attachment(&format!("cart_digikey_{}.xlsx", payload.order_id)))
        .body(digikey_cart.to_vec());

    // configure credentials
    let smtp_server = "smtp.gmail.com";
    dotenv().ok();
    let smtp_user = std::env::var("SMTP_USER")
        .expect("SMTP_USER not set in .env");
    let smtp_pass = std::env::var("SMTP_PASS")
        .expect("SMTP_PASS not set in .env");

    let email = Message::builder()
        .from(smtp_user.parse().map_err(|e: lettre::address::AddressError| DataError::Mail(e.to_string()))?)
        .to(recipient.parse().map_err(|e: lettre::address::AddressError| DataError::Mail(e.to_string()))?)
        .subject(&subject)
        .multipart(
            MultiPart::mixed()
                .singlepart(mail_body)
                .singlepart(mouser_bom_att)
                .singlepart(digikey_bom_att)
                .singlepart(mouser_cart_att)
                .singlepart(digikey_cart_att)
    ).map_err(|e| DataError::Mail(e.to_string()))?;
    println!("verifying credentials...");
    let creds = Credentials::new(smtp_user.to_string(), smtp_pass.to_string());
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_server)
        .map_err(|e| DataError::Mail(e.to_string()))?
        .credentials(creds)
        .build();

    mailer.send(email).await.map_err(|e| DataError::Mail(e.to_string()))?;
    println!("Email sent successfully!");

    Ok(())
}

#[derive(Deserialize)]
struct JobStatusResponse {
    status: String,
}

pub async fn wait_for_bom_job_to_finish(order_id: i32, app_state: AppState) -> Result<(), AppError> {
    println!("waiting for bom to finish...");

    loop {
        let res = edit_order::get_generate_bom_job_status_handler(
            State(app_state.clone()), 
            Path(order_id)
        ).await?;

        println!("job status request sent.");
        if !res.status().is_success() {
            eprintln!("Error checking job status: {}", res.status());
            sleep(Duration::from_secs(2)).await;
            continue;
        }
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.map_err(|e| DataError::Internal(e.to_string()))?;
        let body: JobStatusResponse = serde_json::from_slice(&bytes)
            .map_err(|e| DataError::Internal(format!("Failed to parse JSON: {}", e)))?;
        println!("Job status: {}", body.status);

        if body.status == "done" {
            println!("✅ Job completed!");
            break;
        }

        if body.status == "failed" {
            println!("❌ Job failed");
            break;
        }

        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}