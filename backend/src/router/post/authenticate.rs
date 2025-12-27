use rand::{TryRngCore, rngs::OsRng};
use rocket::post;
use rocket::serde::json::Json;
use std::sync::LazyLock;

use crate::public::config::get_config;
use crate::router::AppResult;
use crate::router::claims::claims::Claims;

static FALLBACK_SECRET_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut secret = vec![0u8; 32];
    OsRng
        .try_fill_bytes(&mut secret)
        .expect("Failed to generate random secret key");
    secret
});

pub fn get_jwt_secret_key() -> Vec<u8> {
    match get_config().auth_key.as_ref() {
        Some(auth_key) => auth_key.as_bytes().to_vec(),
        None => FALLBACK_SECRET_KEY.clone(),
    }
}

#[post("/post/authenticate", data = "<password>")]
pub async fn authenticate(password: Json<String>) -> AppResult<Json<String>> {
    let input_password = password.into_inner();
    if input_password == get_config().password {
        let token = Claims::new_admin().encode_with_key(&get_jwt_secret_key());
        Ok(Json(token))
    } else {
        Err(anyhow::anyhow!("Invalid password")
            .context("Authentication failed")
            .into())
    }
}
