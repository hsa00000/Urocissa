use mini_executor::BatchTask;
use rusqlite::params;

use crate::{
    public::{
        constant::DEFAULT_PRIORITY_LIST, db::sqlite::SQLITE, structure::abstract_data::AbstractData,
    },
    tasks::{BATCH_COORDINATOR, batcher::update_expire::UpdateExpireTask},
};

pub struct FlushTreeTask {
    pub insert_list: Vec<AbstractData>,
    pub remove_list: Vec<AbstractData>,
}

impl FlushTreeTask {
    pub fn insert(databases: Vec<AbstractData>) -> Self {
        Self {
            insert_list: databases,
            remove_list: Vec::new(),
        }
    }
    pub fn remove(databases: Vec<AbstractData>) -> Self {
        Self {
            insert_list: Vec::new(),
            remove_list: databases,
        }
    }
}
impl BatchTask for FlushTreeTask {
    fn batch_run(list: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            let mut all_insert_databases = Vec::new();
            let mut all_remove_databases = Vec::new();
            for task in list {
                all_insert_databases.extend(task.insert_list);
                all_remove_databases.extend(task.remove_list);
            }
            flush_tree_task(all_insert_databases, all_remove_databases);
        }
    }
}

fn flush_tree_task(insert_list: Vec<AbstractData>, remove_list: Vec<AbstractData>) {
    let mut conn = SQLITE.pool.get().unwrap();
    let txn = conn.transaction().unwrap();
    {
        // Objects
        let mut stmt_insert_obj =
            txn.prepare("INSERT OR REPLACE INTO nodes (id, kind, size, width, height, ext, ext_type, pending, timestamp, thumbhash, phash, title, created_time, last_modified_time, start_time, end_time) VALUES (?, 'image', ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, NULL, NULL, NULL)").unwrap();
        let mut stmt_del_obj_tags = txn
            .prepare("DELETE FROM nodes_tags WHERE node_id = ?")
            .unwrap();
        let mut stmt_ins_obj_tag = txn
            .prepare("INSERT INTO nodes_tags (node_id, tag) VALUES (?, ?)")
            .unwrap();
        let mut stmt_del_obj_exif = txn.prepare("DELETE FROM exif WHERE node_id = ?").unwrap();
        let mut stmt_ins_obj_exif = txn
            .prepare("INSERT INTO exif (node_id, tag, value) VALUES (?, ?, ?)")
            .unwrap();
        let mut stmt_del_obj_aliases = txn
            .prepare("DELETE FROM aliases WHERE node_id = ?")
            .unwrap();
        let mut stmt_ins_obj_alias = txn
            .prepare("INSERT INTO aliases (node_id, file, modified, scan_time) VALUES (?, ?, ?, ?)")
            .unwrap();
        let mut stmt_del_alb_objs_by_obj = txn
            .prepare("DELETE FROM album_items WHERE item_id = ?")
            .unwrap();
        let mut stmt_ins_alb_obj = txn
            .prepare("INSERT INTO album_items (album_id, item_id) VALUES (?, ?)")
            .unwrap();

        // Albums
        let mut stmt_insert_alb =
            txn.prepare("INSERT OR REPLACE INTO nodes (id, kind, title, created_time, pending, width, height, start_time, end_time, last_modified_time, size, ext, ext_type, timestamp, thumbhash, phash) VALUES (?, 'album', ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, ?, NULL)").unwrap();
        let mut stmt_insert_album_meta =
            txn.prepare("INSERT OR REPLACE INTO album_meta (album_id, cover_id, user_defined_metadata, item_count, item_size) VALUES (?, ?, ?, ?, ?)").unwrap();
        let mut stmt_del_alb_tags = txn
            .prepare("DELETE FROM nodes_tags WHERE node_id = ?")
            .unwrap();
        let mut stmt_ins_alb_tag = txn
            .prepare("INSERT INTO nodes_tags (node_id, tag) VALUES (?, ?)")
            .unwrap();
        let mut stmt_del_alb_shares = txn
            .prepare("DELETE FROM shares WHERE album_id = ?")
            .unwrap();
        let mut stmt_ins_alb_share = txn
            .prepare("INSERT INTO shares (url, album_id, description, password, show_metadata, show_download, show_upload, exp) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .unwrap();

        // Deletion
        let mut stmt_delete_obj = txn
            .prepare("DELETE FROM nodes WHERE id = ? AND kind IN ('image', 'video')")
            .unwrap();
        let mut stmt_delete_alb = txn
            .prepare("DELETE FROM nodes WHERE id = ? AND kind = 'album'")
            .unwrap();
        let mut stmt_delete_alb_objs_by_alb = txn
            .prepare("DELETE FROM album_items WHERE album_id = ?")
            .unwrap();

        for abstract_data in &insert_list {
            match abstract_data {
                AbstractData::Database(database) => {
                    let timestamp = database.compute_timestamp(&DEFAULT_PRIORITY_LIST);

                    stmt_insert_obj
                        .execute(params![
                            database.hash.as_str(),
                            database.size,
                            database.width,
                            database.height,
                            database.ext,
                            database.ext_type,
                            database.pending,
                            timestamp as i64,
                            database.thumbhash,
                            database.phash
                        ])
                        .unwrap();

                    stmt_del_obj_tags
                        .execute(params![database.hash.as_str()])
                        .unwrap();
                    for tag in &database.tag {
                        stmt_ins_obj_tag
                            .execute(params![database.hash.as_str(), tag])
                            .unwrap();
                    }

                    stmt_del_obj_exif
                        .execute(params![database.hash.as_str()])
                        .unwrap();
                    for (tag, value) in &database.exif_vec {
                        stmt_ins_obj_exif
                            .execute(params![database.hash.as_str(), tag, value])
                            .unwrap();
                    }

                    stmt_del_obj_aliases
                        .execute(params![database.hash.as_str()])
                        .unwrap();
                    for alias in &database.alias {
                        stmt_ins_obj_alias
                            .execute(params![
                                database.hash.as_str(),
                                alias.file,
                                alias.modified as i64,
                                alias.scan_time as i64
                            ])
                            .unwrap();
                    }

                    stmt_del_alb_objs_by_obj
                        .execute(params![database.hash.as_str()])
                        .unwrap();
                    for album_id in &database.album {
                        stmt_ins_alb_obj
                            .execute(params![album_id.as_str(), database.hash.as_str()])
                            .unwrap();
                    }
                }
                AbstractData::Album(album) => {
                    let user_meta_json =
                        serde_json::to_string(&album.user_defined_metadata).unwrap_or_default();
                    let cover = album.cover.map(|c| c.to_string());

                    stmt_insert_alb
                        .execute(params![
                            album.id.as_str(),
                            album.title,
                            album.created_time as i64,
                            album.pending,
                            album.width,
                            album.height,
                            album.start_time.map(|t| t as i64),
                            album.end_time.map(|t| t as i64),
                            album.last_modified_time as i64,
                            album.thumbhash
                        ])
                        .unwrap();

                    stmt_insert_album_meta
                        .execute(params![
                            album.id.as_str(),
                            cover,
                            user_meta_json,
                            album.item_count,
                            album.item_size as i64
                        ])
                        .unwrap();

                    stmt_del_alb_tags
                        .execute(params![album.id.as_str()])
                        .unwrap();
                    for tag in &album.tag {
                        stmt_ins_alb_tag
                            .execute(params![album.id.as_str(), tag])
                            .unwrap();
                    }

                    stmt_del_alb_shares
                        .execute(params![album.id.as_str()])
                        .unwrap();
                    for (url, share) in &album.share_list {
                        stmt_ins_alb_share
                            .execute(params![
                                url.as_str(),
                                album.id.as_str(),
                                share.description,
                                share.password,
                                share.show_metadata,
                                share.show_download,
                                share.show_upload,
                                share.exp as i64
                            ])
                            .unwrap();
                    }
                }
            }
        }

        for abstract_data in &remove_list {
            match abstract_data {
                AbstractData::Database(database) => {
                    stmt_delete_obj
                        .execute(params![database.hash.as_str()])
                        .unwrap();
                    stmt_del_obj_tags
                        .execute(params![database.hash.as_str()])
                        .unwrap();
                    stmt_del_alb_objs_by_obj
                        .execute(params![database.hash.as_str()])
                        .unwrap();
                }
                AbstractData::Album(album) => {
                    stmt_delete_alb.execute(params![album.id.as_str()]).unwrap();
                    stmt_del_alb_tags
                        .execute(params![album.id.as_str()])
                        .unwrap();
                    stmt_delete_alb_objs_by_alb
                        .execute(params![album.id.as_str()])
                        .unwrap();
                }
            }
        }
    }
    txn.commit().unwrap();

    BATCH_COORDINATOR.execute_batch_detached(UpdateExpireTask);
}
