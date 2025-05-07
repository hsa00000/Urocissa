use arrayvec::ArrayString;
use rand::{TryRngCore, rngs::OsRng};
use rocket::post;
use rocket::serde::json::Json;
use std::{
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{public::config::PRIVATE_CONFIG, router::fairing::guard_share::Claims};

pub static JSON_WEB_TOKEN_SECRET_KEY: LazyLock<Vec<u8>> =
    LazyLock::new(|| match PRIVATE_CONFIG.auth_key.as_ref() {
        Some(auth_key) => auth_key.as_bytes().to_vec(),
        _ => {
            let mut secret = vec![0u8; 32];
            OsRng
                .try_fill_bytes(&mut secret)
                .expect("Failed to generate random secret key");
            secret
        }
    });

#[post("/post/authenticate", data = "<password>")]
pub async fn authenticate(password: Json<String>) -> Result<Json<String>, &'static str> {
    let input_password = password.into_inner();

    // Verify the password
    if input_password == PRIVATE_CONFIG.password {
        // Generate token
        let token = Claims::new().encode();

        return Ok(Json(token)); // Return the JWT token
    }

    Err("Invalid password")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareClaims {
    url: ArrayString<64>,
    exp: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareAuthentication {
    url: ArrayString<64>,
    password: Option<String>,
}

#[post("/post/authenticate-share", data = "<album_authentication>")]
pub async fn authenticate_share(
    album_authentication: Json<ShareAuthentication>,
) -> Result<Json<String>, &'static str> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;

    let _claims = ShareClaims {
        url: album_authentication.url,
        exp: expiration as usize,
    };
    todo!()
}
