use super::Expression;
use crate::public::structure::abstract_data::AbstractData;

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
            Expression::Tag(tag) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(img) => img.object.tags.contains(&tag),
                    AbstractData::Video(vid) => vid.object.tags.contains(&tag),
                    AbstractData::Album(alb) => alb.object.tags.contains(&tag),
                })
            }
            Expression::Favorite(value) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(img) => img.object.is_favorite == value,
                    AbstractData::Video(vid) => vid.object.is_favorite == value,
                    AbstractData::Album(alb) => alb.object.is_favorite == value,
                })
            }
            Expression::Archived(value) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(img) => img.object.is_archived == value,
                    AbstractData::Video(vid) => vid.object.is_archived == value,
                    AbstractData::Album(alb) => alb.object.is_archived == value,
                })
            }
            Expression::Trashed(value) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(img) => img.object.is_trashed == value,
                    AbstractData::Video(vid) => vid.object.is_trashed == value,
                    AbstractData::Album(alb) => alb.object.is_trashed == value,
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
                    AbstractData::Image(img) => {
                        img.metadata.ext.to_ascii_lowercase().contains(&ext_lower)
                    }
                    AbstractData::Video(vid) => {
                        vid.metadata.ext.to_ascii_lowercase().contains(&ext_lower)
                    }
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Model(model) => {
                let model_lower = model.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(img) => {
                        img.metadata.exif_vec.get("Model").map_or(false, |model_of_exif| {
                            model_of_exif.to_ascii_lowercase().contains(&model_lower)
                        })
                    }
                    AbstractData::Video(vid) => {
                        vid.metadata.exif_vec.get("Model").map_or(false, |model_of_exif| {
                            model_of_exif.to_ascii_lowercase().contains(&model_lower)
                        })
                    }
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Make(make) => {
                let make_lower = make.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(img) => {
                        img.metadata.exif_vec.get("Make").map_or(false, |make_of_exif| {
                            make_of_exif.to_ascii_lowercase().contains(&make_lower)
                        })
                    }
                    AbstractData::Video(vid) => {
                        vid.metadata.exif_vec.get("Make").map_or(false, |make_of_exif| {
                            make_of_exif.to_ascii_lowercase().contains(&make_lower)
                        })
                    }
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Path(path) => {
                let path_lower = path.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(img) => img.metadata.alias.iter().any(|file_modify| {
                        file_modify.file.to_ascii_lowercase().contains(&path_lower)
                    }),
                    AbstractData::Video(vid) => vid.metadata.alias.iter().any(|file_modify| {
                        file_modify.file.to_ascii_lowercase().contains(&path_lower)
                    }),
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Album(album_id) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(img) => img.metadata.albums.contains(&album_id),
                    AbstractData::Video(vid) => vid.metadata.albums.contains(&album_id),
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Any(any_identifier) => {
                let any_lower = any_identifier.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::Image(img) => {
                        img.object.tags.contains(&any_identifier)
                            || "image".contains(&any_identifier)
                            || img.metadata.ext.to_ascii_lowercase().contains(&any_lower)
                            || img.metadata.exif_vec.get("Make").map_or(false, |make_of_exif| {
                                make_of_exif.to_ascii_lowercase().contains(&any_lower)
                            })
                            || img.metadata.exif_vec.get("Model").map_or(false, |model_of_exif| {
                                model_of_exif.to_ascii_lowercase().contains(&any_lower)
                            })
                            || img.metadata.alias.iter().any(|file_modify| {
                                file_modify.file.to_ascii_lowercase().contains(&any_lower)
                            })
                    }
                    AbstractData::Video(vid) => {
                        vid.object.tags.contains(&any_identifier)
                            || "video".contains(&any_identifier)
                            || vid.metadata.ext.to_ascii_lowercase().contains(&any_lower)
                            || vid.metadata.exif_vec.get("Make").map_or(false, |make_of_exif| {
                                make_of_exif.to_ascii_lowercase().contains(&any_lower)
                            })
                            || vid.metadata.exif_vec.get("Model").map_or(false, |model_of_exif| {
                                model_of_exif.to_ascii_lowercase().contains(&any_lower)
                            })
                            || vid.metadata.alias.iter().any(|file_modify| {
                                file_modify.file.to_ascii_lowercase().contains(&any_lower)
                            })
                    }
                    AbstractData::Album(alb) => {
                        alb.object.tags.contains(&any_identifier)
                            || "album".to_ascii_lowercase().contains(&any_lower)
                    }
                })
            }
        }
    }
}
