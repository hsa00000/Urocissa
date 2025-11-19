use std::time::{SystemTime, UNIX_EPOCH};

use crate::public::db::sqlite::SQLITE;
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

    pub fn self_update(&mut self) {
        let (count, size, start, end, first_db) = SQLITE.get_album_stats(&self.id).unwrap_or((0, 0, None, None, None));

        self.item_count = count;
        self.item_size = size;
        self.start_time = start;
        self.end_time = end;
        self.last_modified_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        if count == 0 {
            self.cover = None;
            self.thumbhash = None;
            self.width = 0;
            self.height = 0;
            return;
        }

        // Check if current cover is valid
        let current_cover_valid = if let Some(cover_id) = &self.cover {
             SQLITE.is_object_in_album(cover_id, &self.id).unwrap_or(false)
        } else {
            false
        };

        if !current_cover_valid {
            if let Some(db) = first_db {
                self.set_cover(&db);
            }
        }
    }
}
