use super::Expression;
use crate::public::structure::abstract_data::AbstractData;
use crate::table::database::MediaCombined;
use arrayvec::ArrayString;

impl Expression {
    pub fn generate_filter_hide_metadata(
        self,
        shared_album_id: ArrayString<64>,
    ) -> Box<dyn Fn(&AbstractData) -> bool + Send + Sync> {
        match self {
            Expression::Or(exprs) => {
                let id = shared_album_id.clone();
                let filters = exprs;
                Box::new(move |data| {
                    filters.iter().any(|expr| {
                        let filter = expr.clone().generate_filter_hide_metadata(id.clone());
                        filter(data)
                    })
                })
            }
            Expression::And(exprs) => {
                let id = shared_album_id.clone();
                let filters = exprs;
                Box::new(move |data| {
                    filters.iter().all(|expr| {
                        let filter = expr.clone().generate_filter_hide_metadata(id.clone());
                        filter(data)
                    })
                })
            }
            Expression::Not(expr) => {
                let inner = expr.generate_filter_hide_metadata(shared_album_id);
                Box::new(move |data| !inner(data))
            }

            /* ---------- Allowed album condition ---------- */
            Expression::Album(album_id) => {
                if album_id == shared_album_id {
                    Box::new(move |data| match data {
                        AbstractData::Media(media) => {
                            // 媒體的 album 關聯需要從關聯表查詢
                            // 這裡簡化，返回 false，因為媒體的 album 關聯是動態的
                            false
                        }
                        AbstractData::Database(db) => db.album.contains(&album_id),
                        AbstractData::Album(_) => false,
                    })
                } else {
                    // Not the shared album ID → always invalid
                    Box::new(|_| false)
                }
            }

            /* ---------- Supplementary conditions that must be invalid ---------- */
            Expression::Tag(_) | Expression::Path(_) => Box::new(|_| false),

            /* ---------- Still allowed embedded / file-related conditions ---------- */
            Expression::ExtType(ext_type) => Box::new(move |data| match data {
                AbstractData::Media(media) => match media {
                    MediaCombined::Image(_) => ext_type.contains("image"),
                    MediaCombined::Video(_) => ext_type.contains("video"),
                },
                AbstractData::Database(db) => db.schema.ext_type.contains(&ext_type),
                AbstractData::Album(_) => false,
            }),
            Expression::Ext(ext) => {
                let ext_lower = ext.to_ascii_lowercase();
                Box::new(move |data| match data {
                    AbstractData::Media(media) => match media {
                        MediaCombined::Image(i) => i.metadata.ext.to_ascii_lowercase().contains(&ext_lower),
                        MediaCombined::Video(v) => v.metadata.ext.to_ascii_lowercase().contains(&ext_lower),
                    },
                    AbstractData::Database(db) => {
                        db.schema.ext.to_ascii_lowercase().contains(&ext_lower)
                    }
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Model(_model) => {
                /* let model_lower = model.to_ascii_lowercase();
                Box::new(move |data| match data {
                    AbstractData::DatabaseSchema(db) => db
                        .exif_vec
                        .get("Model")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&model_lower)),
                    AbstractData::Album(_) => false,
                }) */
                todo!()
            }
            Expression::Make(_make) => {
                /*  let make_lower = make.to_ascii_lowercase();
                Box::new(move |data| match data {
                    AbstractData::DatabaseSchema(db) => db
                        .exif_vec
                        .get("Make")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&make_lower)),
                    AbstractData::Album(_) => false,
                }) */
                todo!()
            }

            /* ---------- Any: removes tag / alias / album / path matching ---------- */
            Expression::Any(_identifier) => {
                /*  let any_lower = identifier.to_ascii_lowercase();
                Box::new(move |data| match data {
                    AbstractData::DatabaseSchema(db) => {
                        db.ext_type.contains(&identifier)
                            || db.ext.to_ascii_lowercase().contains(&any_lower)
                            || db
                                .exif_vec
                                .get("Make")
                                .map_or(false, |v| v.to_ascii_lowercase().contains(&any_lower))
                            || db
                                .exif_vec
                                .get("Model")
                                .map_or(false, |v| v.to_ascii_lowercase().contains(&any_lower))
                    }
                    AbstractData::Album(_) => false,
                }) */
                todo!()
            }
        }
    }
}
