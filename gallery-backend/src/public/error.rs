use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder};
use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    InvalidInput,
    Internal,
    Conflict,
    Database,
    IO,
    #[allow(dead_code)]
    Serialization,
    Auth,
    ReadOnlyMode,
    Unavailable,
}

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorKind::NotFound => "Not Found",
            ErrorKind::PermissionDenied => "Permission Denied",
            ErrorKind::InvalidInput => "Invalid Input",
            ErrorKind::Internal => "Internal Error",
            ErrorKind::Conflict => "Conflict",
            ErrorKind::Database => "Database Error",
            ErrorKind::IO => "I/O Error",
            ErrorKind::Serialization => "Serialization Error",
            ErrorKind::Auth => "Authentication Error",
            ErrorKind::ReadOnlyMode => "Read Only Mode",
            ErrorKind::Unavailable => "Service Unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ErrorStatus {
    Permanent,
    #[allow(dead_code)]
    Temporary,
}

#[derive(Debug)]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
    pub status: ErrorStatus,
    pub context: Vec<String>,
    pub source: Option<anyhow::Error>,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 4)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("status", &self.status)?;
        state.serialize_field("context", &self.context)?;
        state.end()
    }
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            status: ErrorStatus::Permanent,
            context: Vec::new(),
            source: None,
        }
    }

    #[allow(dead_code)]
    pub fn temporary(mut self) -> Self {
        self.status = ErrorStatus::Temporary;
        self
    }

    pub fn context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }

    pub fn from_err(kind: ErrorKind, err: anyhow::Error) -> Self {
        Self {
            kind,
            message: err.to_string(),
            status: ErrorStatus::Permanent,
            context: Vec::new(),
            source: Some(err),
        }
    }

    pub fn http_status(&self) -> Status {
        match self.kind {
            ErrorKind::NotFound => Status::NotFound,
            ErrorKind::PermissionDenied => Status::Forbidden,
            ErrorKind::Auth => Status::Unauthorized,
            ErrorKind::InvalidInput => Status::BadRequest,
            ErrorKind::Conflict => Status::Conflict,
            ErrorKind::ReadOnlyMode => Status::MethodNotAllowed,
            ErrorKind::Unavailable => Status::ServiceUnavailable,
            _ => Status::InternalServerError,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.message)?;
        for ctx in &self.context {
            write!(f, "\n  Context: {ctx}")?;
        }
        if let Some(src) = &self.source {
            write!(f, "\n  Caused by: {src:?}")?;
        }
        Ok(())
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(anyhow::Error::root_cause)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::from_err(ErrorKind::Internal, err)
    }
}

// Rocket Responder
impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let status = self.http_status();

        // Log the error
        log::error!("API Error: {self}");

        // Also trigger the old handler for Discord notifications if needed
        crate::public::error_data::handle_app_error(&self);

        let is_unavailable = self.kind == ErrorKind::Unavailable;
        rocket::serde::json::Json(self)
            .respond_to(req)
            .map(|mut res| {
                res.set_status(status);
                if is_unavailable {
                    res.set_raw_header("Retry-After", "30");
                }
                res
            })
    }
}

// Extension trait for Result to easily convert to AppError with context
pub trait ResultExt<T> {
    fn or_raise<F, S>(self, op: F) -> Result<T, AppError>
    where
        F: FnOnce() -> (ErrorKind, S),
        S: Into<String>;

    #[allow(dead_code)]
    fn with_context<S>(self, ctx: S) -> Result<T, AppError>
    where
        S: Into<String>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn or_raise<F, S>(self, op: F) -> Result<T, AppError>
    where
        F: FnOnce() -> (ErrorKind, S),
        S: Into<String>,
    {
        self.map_err(|e| {
            let (kind, msg) = op();
            AppError {
                kind,
                message: msg.into(),
                status: ErrorStatus::Permanent,
                context: vec![],
                source: Some(e.into()),
            }
        })
    }

    fn with_context<S>(self, ctx: S) -> Result<T, AppError>
    where
        S: Into<String>,
    {
        self.map_err(|e| AppError {
            kind: ErrorKind::Internal,
            message: "An unexpected error occurred".to_string(),
            status: ErrorStatus::Permanent,
            context: vec![ctx.into()],
            source: Some(e.into()),
        })
    }
}

// Extension trait for Option
#[allow(dead_code)]
pub trait OptionExt<T> {
    fn or_raise<F, S>(self, op: F) -> Result<T, AppError>
    where
        F: FnOnce() -> (ErrorKind, S),
        S: Into<String>;
}

impl<T> OptionExt<T> for Option<T> {
    fn or_raise<F, S>(self, op: F) -> Result<T, AppError>
    where
        F: FnOnce() -> (ErrorKind, S),
        S: Into<String>,
    {
        self.ok_or_else(|| {
            let (kind, msg) = op();
            AppError::new(kind, msg)
        })
    }
}
