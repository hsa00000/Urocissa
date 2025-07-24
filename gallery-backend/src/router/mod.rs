pub mod claims;
pub mod delete;
pub mod fairing;
pub mod get;
pub mod post;
pub mod put;
use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use serde_json::json;
use std::io::Cursor;

#[derive(Debug)]
pub struct AppError(anyhow::Error);

#[rocket::async_trait]
impl<'r, 'o: 'r> Responder<'r, 'o> for AppError {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'o> {
        let outer_msg = self.0.to_string();

        let chain: Vec<String> = self.0.chain().map(|e| e.to_string()).collect();

        let body = json!({
            "error": outer_msg,
            "chain": chain,
        })
        .to_string();

        Response::build()
            .status(Status::InternalServerError)
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
        AppError(anyhow::Error::from(err))
    }
}

pub type AppResult<T> = Result<T, AppError>;
