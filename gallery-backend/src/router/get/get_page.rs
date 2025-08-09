use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::response::{Redirect, content};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

pub static INDEX_HTML: LazyLock<String> = LazyLock::new(|| {
    fs::read_to_string("../gallery-frontend/dist/index.html").expect("Unable to read index.html")
});

#[get("/")]
pub fn redirect_to_photo() -> content::RawHtml<String> {
    content::RawHtml(INDEX_HTML.to_string())
}

#[get("/login")]
pub async fn login() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/redirect-to-login")]
pub fn redirect_to_login() -> Redirect {
    Redirect::to(uri!("/login"))
}

#[get("/unauthorized")]
pub async fn unauthorized() -> Status {
    Status::Unauthorized
}

#[get("/home")]
pub async fn home() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/view/<_path..>")]
pub async fn view(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/favorite")]
pub async fn favorite() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/albums")]
pub async fn albums() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/<dynamic_album_id>")]
pub async fn album_page(dynamic_album_id: String) -> Option<NamedFile> {
    if dynamic_album_id.starts_with("album-") {
        NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
            .await
            .ok()
    } else {
        None
    }
}

#[get("/share/<_path..>")]
pub async fn share(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/archived")]
pub async fn archived() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/trashed")]
pub async fn trashed() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/all")]
pub async fn all() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/videos")]
pub async fn videos() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/tags")]
pub async fn tags() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/links")]
pub async fn links() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/setting")]
pub async fn setting() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/favicon.ico")]
pub async fn favicon() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/favicon.ico"))
        .await
        .ok()
}

#[get("/registerSW.js")]
pub async fn sregister_sw() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/registerSW.js"))
        .await
        .ok()
}

#[get("/serviceWorker.js")]
pub async fn service_worker() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/serviceWorker.js"))
        .await
        .ok()
}
