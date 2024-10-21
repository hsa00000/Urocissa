use rocket::fs::NamedFile;
use rocket::response::{content, Redirect};
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

#[get("/share/<_path..>")]
pub fn redirect_to_photo_2(_path: PathBuf) -> content::RawHtml<String> {
    println!("RETURNING");
    content::RawHtml(INDEX_HTML.to_string())
}

#[get("/login")]
pub async fn login() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/login.html"))
        .await
        .ok()
}

#[get("/redirect-to-login")]
pub async fn redirect_to_login() -> Redirect {
    Redirect::to(uri!("/login"))
}

#[get("/view/<_path..>")]
pub async fn catch_view_routes(_path: PathBuf) -> Option<NamedFile> {
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

#[get("/favorite")]
pub async fn favorite() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/favorite/view/<_hash>")]
pub async fn favorite_view(_hash: String) -> Option<NamedFile> {
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

#[get("/archived/view/<_hash>")]
pub async fn archived_view(_hash: String) -> Option<NamedFile> {
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

#[get("/trashed/view/<_hash>")]
pub async fn trashed_view(_hash: String) -> Option<NamedFile> {
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

#[get("/all/view/<_hash>")]
pub async fn all_view(_hash: String) -> Option<NamedFile> {
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