use rocket::post;
use rocket::serde::json::Json;
// 移除 rand, LazyLock, get_config 等舊引入

use crate::public::structure::config::APP_CONFIG;
use crate::router::AppResult;
use crate::router::claims::claims::Claims;

#[post("/post/authenticate", data = "<password>")]
pub async fn authenticate(password: Json<String>) -> AppResult<Json<String>> {
    let input_password = password.into_inner();

    let current_password = APP_CONFIG.get().unwrap().read().unwrap().password.clone();

    if input_password == current_password {
        let token = Claims::new_admin().encode();
        Ok(Json(token))
    } else {
        Err(anyhow::anyhow!("Invalid password")
            .context("Authentication failed")
            .into())
    }
}
