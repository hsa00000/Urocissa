use std::sync::LazyLock;
pub mod expired_check;
pub mod new;

#[derive(Debug)]
pub struct Expire;

pub static EXPIRE: LazyLock<Expire> = LazyLock::new(|| Expire::new());

