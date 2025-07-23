use crate::{
    public::constant::redb::ALBUM_TABLE,
    public::structure::album::Album,
};

use super::Tree;
use redb::ReadOnlyTable;

impl Tree {
    pub fn api_read_album(&self) -> ReadOnlyTable<&str, Album> {
        self.in_disk
            .begin_read()
            .unwrap()
            .open_table(ALBUM_TABLE)
            .unwrap()
    }
}
