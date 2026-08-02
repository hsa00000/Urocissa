#[cfg(not(feature = "embed-frontend"))]
use crate::public::error::ResultExt;
use crate::public::error::{AppError, ErrorKind};
use crate::router::AppResult;
#[cfg(not(feature = "embed-frontend"))]
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::response::{Redirect, content};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

#[cfg(feature = "embed-frontend")]
use crate::public::embedded::FrontendAssets;
#[cfg(feature = "embed-frontend")]
use rocket::http::ContentType;
#[cfg(feature = "embed-frontend")]
use std::borrow::Cow;

pub static INDEX_HTML: LazyLock<String> = LazyLock::new(|| {
    #[cfg(feature = "embed-frontend")]
    {
        if let Some(file) = FrontendAssets::get("index.html") {
            return std::str::from_utf8(&file.data)
                .expect("index.html is not valid UTF-8")
                .to_string();
        }
    }

    let prod_path = Path::new("index.html");
    if prod_path.exists() {
        fs::read_to_string(prod_path).expect("Unable to read index.html from current directory")
    } else {
        fs::read_to_string("../gallery-frontend/dist/index.html")
            .expect("Unable to read index.html from dev path")
    }
});

#[cfg(not(feature = "embed-frontend"))]
fn resolve_path(filename: &str) -> PathBuf {
    let prod_path = Path::new(filename);
    if prod_path.exists() {
        prod_path.to_path_buf()
    } else {
        PathBuf::from(format!("../gallery-frontend/dist/{filename}"))
    }
}

// Custom responder that can return either a NamedFile or embedded content
pub enum FrontendResponse {
    #[cfg(not(feature = "embed-frontend"))]
    File(NamedFile),
    #[cfg(feature = "embed-frontend")]
    Embedded(ContentType, Cow<'static, [u8]>),
}

impl<'r> rocket::response::Responder<'r, 'static> for FrontendResponse {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        match self {
            #[cfg(not(feature = "embed-frontend"))]
            FrontendResponse::File(f) => f.respond_to(request),
            #[cfg(feature = "embed-frontend")]
            FrontendResponse::Embedded(ct, data) => {
                let _ = request;
                rocket::response::Response::build()
                    .header(ct)
                    .sized_body(data.len(), std::io::Cursor::new(data))
                    .ok()
            }
        }
    }
}

// Helper to serve file (either from disk or embedded)
#[allow(clippy::unused_async)]
async fn serve_file(filename: &str) -> AppResult<FrontendResponse> {
    #[cfg(feature = "embed-frontend")]
    {
        if let Some(asset) = FrontendAssets::get(filename) {
            let mime = mime_guess::from_path(filename).first_or_octet_stream();
            let ct = ContentType::parse_flexible(mime.as_ref()).unwrap_or(ContentType::Binary);
            return Ok(FrontendResponse::Embedded(ct, asset.data));
        }
        // If not found in embedded, fallback to error (or disk if you want mixed mode)
        Err(AppError::new(
            ErrorKind::NotFound,
            format!("Embedded file not found: {filename}"),
        ))
    }

    #[cfg(not(feature = "embed-frontend"))]
    {
        let path = resolve_path(filename);
        NamedFile::open(path)
            .await
            .map(FrontendResponse::File)
            .or_raise(|| (ErrorKind::IO, format!("Failed to open {filename}")))
    }
}

#[get("/")]
pub fn redirect_to_photo() -> content::RawHtml<String> {
    content::RawHtml(INDEX_HTML.to_string())
}

#[get("/login")]
pub async fn login() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/redirect-to-login")]
pub fn redirect_to_login() -> Redirect {
    Redirect::to(uri!("/login"))
}

#[get("/unauthorized")]
pub fn unauthorized() -> Status {
    Status::Unauthorized
}

#[get("/home")]
pub async fn home() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/upload")]
pub async fn upload() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/home/view/<_path..>")]
pub async fn home_view(_path: PathBuf) -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/favorite")]
pub async fn favorite() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/favorite/view/<_path..>")]
pub async fn favorite_view(_path: PathBuf) -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/albums")]
pub async fn albums() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/albums/view/<_path..>")]
pub async fn albums_view(_path: PathBuf) -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/<dynamic_album_id>")]
pub async fn album_page(dynamic_album_id: String) -> AppResult<FrontendResponse> {
    if dynamic_album_id.starts_with("album-") {
        serve_file("index.html").await
    } else {
        Err(AppError::new(ErrorKind::NotFound, "Page not found"))
    }
}

#[get("/share/<_path..>")]
pub async fn share(_path: PathBuf) -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/archived")]
pub async fn archived() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/archived/view/<_path..>")]
pub async fn archived_view(_path: PathBuf) -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/trashed")]
pub async fn trashed() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/trashed/view/<_path..>")]
pub async fn trashed_view(_path: PathBuf) -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/all")]
pub async fn all() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/all/view/<_path..>")]
pub async fn all_view(_path: PathBuf) -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/videos")]
pub async fn videos() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/videos/view/<_path..>")]
pub async fn videos_view(_path: PathBuf) -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/tags")]
pub async fn tags() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/links")]
pub async fn links() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/config")]
pub async fn config() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/setting")]
pub async fn setting() -> AppResult<FrontendResponse> {
    serve_file("index.html").await
}

#[get("/favicon.ico")]
pub async fn favicon() -> AppResult<FrontendResponse> {
    serve_file("favicon.ico").await
}

#[get("/registerSW.js")]
pub async fn sregister_sw() -> AppResult<FrontendResponse> {
    serve_file("registerSW.js").await
}

#[get("/serviceWorker.js")]
pub async fn service_worker() -> AppResult<FrontendResponse> {
    serve_file("serviceWorker.js").await
}
