use super::Expression;
use crate::models::entity::abstract_data::AbstractData;

impl Expression {
    pub fn generate_filter(self) -> Box<dyn Fn(&AbstractData) -> bool + Sync + Send> {
        match self {
            Expression::Or(expressions) => {
                let filters: Vec<Expression> = expressions;
                Box::new(move |abstract_data: &AbstractData| {
                    filters.iter().any(|expr| {
                        let filter = expr.clone().generate_filter();
                        filter(abstract_data)
                    })
                })
            }
            Expression::And(expressions) => {
                let filters: Vec<Expression> = expressions;
                Box::new(move |abstract_data: &AbstractData| {
                    filters.iter().all(|expr| {
                        let filter = expr.clone().generate_filter();
                        filter(abstract_data)
                    })
                })
            }
            Expression::Not(expression) => {
                let inner_filter = expression.clone().generate_filter();
                Box::new(move |abstract_data: &AbstractData| !inner_filter(abstract_data))
            }
            Expression::Tag(tag) => Box::new(move |abstract_data: &AbstractData| {
                let tags = match abstract_data {
                    AbstractData::Image(image) => &image.object.tags,
                    AbstractData::Video(video) => &video.object.tags,
                    AbstractData::Album(album) => &album.object.tags,
                };
                tags.contains(&tag)
            }),
            Expression::Favorite(value) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(image) => image.object.is_favorite == value,
                    AbstractData::Video(video) => video.object.is_favorite == value,
                    AbstractData::Album(album) => album.object.is_favorite == value,
                })
            }
            Expression::Archived(value) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(image) => image.object.is_archived == value,
                    AbstractData::Video(video) => video.object.is_archived == value,
                    AbstractData::Album(album) => album.object.is_archived == value,
                })
            }
            Expression::Trashed(value) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(image) => image.object.is_trashed == value,
                    AbstractData::Video(video) => video.object.is_trashed == value,
                    AbstractData::Album(album) => album.object.is_trashed == value,
                })
            }
            Expression::ExtType(ext_type) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(_) => ext_type.contains("image"),
                    AbstractData::Video(_) => ext_type.contains("video"),
                    AbstractData::Album(_) => ext_type.contains("album"),
                })
            }
            Expression::Ext(ext) => {
                let ext_lower = ext.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(image) => {
                        image.metadata.ext.to_ascii_lowercase().contains(&ext_lower)
                    }
                    AbstractData::Video(video) => {
                        video.metadata.ext.to_ascii_lowercase().contains(&ext_lower)
                    }
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Model(model) => {
                let model_lower = model.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(image) => image
                        .exif_vec
                        .get("Model")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&model_lower)),
                    AbstractData::Video(video) => video
                        .exif_vec
                        .get("Model")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&model_lower)),
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Make(make) => {
                let make_lower = make.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(image) => image
                        .exif_vec
                        .get("Make")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&make_lower)),
                    AbstractData::Video(video) => video
                        .exif_vec
                        .get("Make")
                        .map_or(false, |v| v.to_ascii_lowercase().contains(&make_lower)),
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Path(path) => {
                let path_lower = path.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| {
                    // alias() 會去查資料庫，可能較慢，但在 Path 搜尋中是必須的
                    abstract_data.alias().iter().any(|file_modify| {
                        file_modify.file.to_ascii_lowercase().contains(&path_lower)
                    })
                })
            }
            Expression::Album(album_id) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(image) => image.albums.contains(&album_id),
                    AbstractData::Video(video) => video.albums.contains(&album_id),
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Any(any_identifier) => {
                let any_lower = any_identifier.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(image) => {
                        image.object.tags.contains(&any_identifier)
                            || "image".contains(&any_lower)
                            || image.metadata.ext.to_ascii_lowercase().contains(&any_lower)
                            || image.exif_vec.get("Make").map_or(false, |v| {
                                v.to_ascii_lowercase().contains(&any_lower)
                            })
                            || image.exif_vec.get("Model").map_or(false, |v| {
                                v.to_ascii_lowercase().contains(&any_lower)
                            })
                            // 將 alias 檢查放在最後，因為需要讀取資料庫
                            || abstract_data.alias().iter().any(|file_modify| {
                                file_modify.file.to_ascii_lowercase().contains(&any_lower)
                            })
                    }
                    AbstractData::Video(video) => {
                        video.object.tags.contains(&any_identifier)
                            || "video".contains(&any_lower)
                            || video.metadata.ext.to_ascii_lowercase().contains(&any_lower)
                            || video
                                .exif_vec
                                .get("Make")
                                .map_or(false, |v| v.to_ascii_lowercase().contains(&any_lower))
                            || video
                                .exif_vec
                                .get("Model")
                                .map_or(false, |v| v.to_ascii_lowercase().contains(&any_lower))
                            || abstract_data.alias().iter().any(|file_modify| {
                                file_modify.file.to_ascii_lowercase().contains(&any_lower)
                            })
                    }
                    AbstractData::Album(album) => {
                        album.object.tags.contains(&any_identifier) || "album".contains(&any_lower)
                    }
                })
            }
        }
    }
}
