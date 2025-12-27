use crate::public::structure::album::ResolvedShare;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Admin,
    Share(ResolvedShare),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub role: Role,
    pub exp: u64,
}

impl Claims {
    pub fn new_admin() -> Self {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + 14 * 86_400; // 14 days

        Self {
            role: Role::Admin,
            exp,
        }
    }

    pub fn new_share(resolved_share: ResolvedShare) -> Self {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + 14 * 86_400; // 14 days

        Self {
            role: Role::Share(resolved_share),
            exp,
        }
    }
    pub fn is_admin(&self) -> bool {
        match &self.role {
            Role::Admin => true,
            _ => false,
        }
    }
    pub fn get_share(self) -> Option<ResolvedShare> {
        match self.role {
            Role::Share(share) => Some(share),
            _ => None,
        }
    }

    pub fn encode(&self) -> String {
        // 引入 APP_CONFIG
        use crate::public::structure::config::APP_CONFIG;
        
        // 獲取 ReadGuard 並呼叫方法
        let config = APP_CONFIG.get().unwrap().read().unwrap();
        self.encode_with_key(&config.get_jwt_secret_key())
    }

    pub fn encode_with_key(&self, key: &[u8]) -> String {
        encode(&Header::default(), &self, &EncodingKey::from_secret(key))
            .expect("Failed to generate token")
    }
}
