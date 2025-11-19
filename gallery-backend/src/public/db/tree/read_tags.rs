use crate::public::structure::tag_info::TagInfo;
use crate::public::db::sqlite::SQLITE;
use crate::public::structure::album::Album;
use anyhow::Result;

use super::Tree;

impl Tree {
    pub fn read_tags(&'static self) -> Vec<TagInfo> {
        SQLITE.get_all_tags().unwrap_or_default()
    }
    pub fn read_albums(&self) -> Result<Vec<Album>> {
        Ok(SQLITE.get_all_albums()?)
    }
}
