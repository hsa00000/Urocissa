use super::Expression;
use crate::public::structure::abstract_data::AbstractData;
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
                        AbstractData::Image(img) => img.metadata.albums.contains(&album_id),
                        AbstractData::Video(vid) => vid.metadata.albums.contains(&album_id),
                        AbstractData::Album(_) => false,
                    })
                } else {
                    // Not the shared album ID → always invalid
                    Box::new(|_| false)
                }
            }

            /* ---------- Supplementary conditions that must be invalid ---------- */
            Expression::Tag(_) | Expression::Path(_) => Box::new(|_| false),
            
            /* ---------- Boolean field filters ---------- */
            Expression::Favorite(value) => {
                Box::new(move |data: &AbstractData| match data {
                    AbstractData::Image(img) => img.object.is_favorite == value,
                    AbstractData::Video(vid) => vid.object.is_favorite == value,
                    AbstractData::Album(alb) => alb.object.is_favorite == value,
                })
            }
            Expression::Archived(value) => {
                Box::new(move |data: &AbstractData| match data {
                    AbstractData::Image(img) => img.object.is_archived == value,
                    AbstractData::Video(vid) => vid.object.is_archived == value,
                    AbstractData::Album(alb) => alb.object.is_archived == value,
                })
            }
            Expression::Trashed(value) => {
                Box::new(move |data: &AbstractData| match data {
                    AbstractData::Image(img) => img.object.is_trashed == value,
                    AbstractData::Video(vid) => vid.object.is_trashed == value,
                    AbstractData::Album(alb) => alb.object.is_trashed == value,
                })
            }

            /* ---------- Still allowed embedded / file-related conditions ---------- */
            Expression::ExtType(ext_type) => Box::new(move |data| match data {
                AbstractData::Image(_) => ext_type.contains("image"),
                AbstractData::Video(_) => ext_type.contains("video"),
                AbstractData::Album(_) => false,
            }),
            Expression::Ext(ext) => {
                let ext_lower = ext.to_ascii_lowercase();
                Box::new(move |data| match data {
                    AbstractData::Image(img) => img.metadata.ext.to_ascii_lowercase().contains(&ext_lower),
                    AbstractData::Video(vid) => vid.metadata.ext.to_ascii_lowercase().contains(&ext_lower),
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Model(model) => {
                let model_lower = model.to_ascii_lowercase();
                Box::new(move |data| match data {
                    AbstractData::Image(img) => img
                        .metadata.exif_vec
                        .get("Model")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&model_lower)),
                    AbstractData::Video(vid) => vid
                        .metadata.exif_vec
                        .get("Model")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&model_lower)),
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Make(make) => {
                let make_lower = make.to_ascii_lowercase();
                Box::new(move |data| match data {
                    AbstractData::Image(img) => img
                        .metadata.exif_vec
                        .get("Make")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&make_lower)),
                    AbstractData::Video(vid) => vid
                        .metadata.exif_vec
                        .get("Make")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&make_lower)),
                    AbstractData::Album(_) => false,
                })
            }

            /* ---------- Any: removes tag / alias / album / path matching ---------- */
            Expression::Any(identifier) => {
                let any_lower = identifier.to_ascii_lowercase();
                Box::new(move |data| match data {
                    AbstractData::Image(img) => {
                        "image".contains(&identifier)
                            || img.metadata.ext.to_ascii_lowercase().contains(&any_lower)
                            || img
                                .metadata.exif_vec
                                .get("Make")
                                .map_or(false, |v| v.to_ascii_lowercase().contains(&any_lower))
                            || img
                                .metadata.exif_vec
                                .get("Model")
                                .map_or(false, |v| v.to_ascii_lowercase().contains(&any_lower))
                    }
                    AbstractData::Video(vid) => {
                        "video".contains(&identifier)
                            || vid.metadata.ext.to_ascii_lowercase().contains(&any_lower)
                            || vid
                                .metadata.exif_vec
                                .get("Make")
                                .map_or(false, |v| v.to_ascii_lowercase().contains(&any_lower))
                            || vid
                                .metadata.exif_vec
                                .get("Model")
                                .map_or(false, |v| v.to_ascii_lowercase().contains(&any_lower))
                    }
                    AbstractData::Album(_) => false,
                })
            }
        }
    }
}
