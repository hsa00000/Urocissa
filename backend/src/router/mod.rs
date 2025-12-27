pub mod claims;
pub mod delete;
pub mod fairing;
pub mod get;
pub mod post;
pub mod put;
pub mod settings;
use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::Route;
use serde_json::json;
use std::io::Cursor;

pub fn generate_settings_routes() -> Vec<Route> {
    routes![
        settings::get_settings,
        settings::update_settings,
        settings::export_settings,
        settings::import_settings,
    ]
}

#[derive(Debug)]
pub struct AppError {
    pub status: Status,
    pub error: anyhow::Error,
}

#[rocket::async_trait]
impl<'r, 'o: 'r> Responder<'r, 'o> for AppError {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'o> {
        let outer_msg = self.error.to_string();

        let chain: Vec<String> = self.error.chain().map(|e| e.to_string()).collect();

        let body = json!({
            "error": outer_msg,
            "chain": chain,
        })
        .to_string();

        Response::build()
            .status(self.status)
            .header(ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}

impl<E> From<E> for AppError
where
    anyhow::Error: From<E>,
{
    fn from(err: E) -> Self {
        AppError {
            status: Status::InternalServerError,
            error: anyhow::Error::from(err),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub struct GuardError {
    pub status: Status,
    pub error: anyhow::Error,
}

impl From<GuardError> for AppError {
    fn from(err: GuardError) -> Self {
        AppError {
            status: err.status,
            error: err.error,
        }
    }
}

pub type GuardResult<T> = Result<T, GuardError>;

impl<E> From<E> for GuardError
where
    anyhow::Error: From<E>,
{
    fn from(err: E) -> Self {
        GuardError {
            status: Status::Unauthorized,
            error: anyhow::Error::from(err),
        }
    }
}
