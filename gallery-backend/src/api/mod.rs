pub mod claims;
pub mod fairings;
pub mod handlers;
use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use serde_json::json;
use std::io::Cursor;

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
            status: err.status, // 使用 GuardError 攜帶的狀態碼
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
        // 預設情況下 (例如 Auth Guard 的一般錯誤)，維持 Unauthorized
        GuardError {
            status: Status::Unauthorized,
            error: anyhow::Error::from(err),
        }
    }
}
