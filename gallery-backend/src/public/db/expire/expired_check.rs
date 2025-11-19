use super::Expire;

impl Expire {
    pub fn expired_check(&self, _timestamp: u64) -> bool {
        false
    }
}

