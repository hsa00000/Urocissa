use crate::public::structure::{
    database_struct::database::definition::Database,
};

use super::Album;

impl Album {
    pub fn set_cover(&mut self, cover_data: &Database) {
        self.cover = Some(cover_data.hash);
        self.thumbhash = Some(cover_data.thumbhash.clone());
        self.width = cover_data.width;
        self.height = cover_data.height;
    }
}
