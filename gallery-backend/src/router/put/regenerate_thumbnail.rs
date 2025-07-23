use crate::operations::indexation::generate_dynamic_image::generate_dynamic_image;
use crate::operations::indexation::generate_image_hash::{generate_phash, generate_thumbhash};
use crate::operations::open_db::open_data_table;
use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::tasks::batcher::flush_tree::FlushTreeTask;

use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::INDEX_COORDINATOR;
use arrayvec::ArrayString;
use rocket::form::Form;
use rocket::form::{self, DataField, FromFormField, ValueField};
use rocket::fs::TempFile;
use rocket::http::Status;
pub enum FrameData<'r> {
    Hash(ArrayString<64>),
    File(TempFile<'r>),
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for FrameData<'r> {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        let valid_hash = String::from_value(field)?;
        match ArrayString::<64>::from(&valid_hash) {
            Ok(hash) => Ok(FrameData::Hash(hash)),
            Err(_) => Err(form::Error::validation("Invalid hash length or format").into()),
        }
    }

    async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
        match TempFile::from_data(field).await {
            Ok(temp_file) => Ok(FrameData::File(temp_file)),
            Err(err) => Err(err),
        }
    }
}

#[put("/put/regenerate-thumbnail-with-frame", data = "<data>")]
pub async fn regenerate_thumbnail_with_frame(
    _auth: GuardAuth,
    _read_only_mode: GuardReadOnlyMode,
    data: Form<Vec<FrameData<'_>>>,
) -> Result<Status, Status> {
    let mut hash: Option<ArrayString<64>> = None;

    for frame_data in data.into_inner() {
        match frame_data {
            FrameData::Hash(received_hash) => {
                hash = Some(received_hash);
            }
            FrameData::File(mut file) => {
                let hash = hash.unwrap();
                let file_path = format!("./object/compressed/{}/{}.jpg", &hash[0..2], hash);

                if let Err(err) = file.move_copy_to(file_path).await {
                    error!("Failed to save file: {:#?}", err);
                    return Err(Status::InternalServerError);
                }
                let abstract_data = tokio::task::spawn_blocking(move || {
                    let data_table = open_data_table();
                    let mut database = data_table.get(&*hash).unwrap().unwrap().value();
                    let dynamic_image = generate_dynamic_image(&database).unwrap();
                    database.thumbhash = generate_thumbhash(&dynamic_image);
                    database.phash = generate_phash(&dynamic_image);
                    AbstractData::Database(database)
                })
                .await
                .unwrap();

                INDEX_COORDINATOR
                    .execute_batch_waiting(FlushTreeTask::insert(vec![abstract_data]))
                    .await
                    .unwrap();
            }
        }
    }

    info!("Regenerating thumbnail successfully");

    Ok(Status::Ok)
}
