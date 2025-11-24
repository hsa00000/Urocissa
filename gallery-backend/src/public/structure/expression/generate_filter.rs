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
            Expression::Tag(tag) => Box::new(move |abstract_data: &AbstractData| {
                abstract_data
                    .tag()
                    .as_ref()
                    .map_or(false, |tags| tags.contains(&tag))
            }),
            Expression::ExtType(ext_type) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::DatabaseSchema(db) => db.ext_type.contains(&ext_type),
                    AbstractData::Album(_) => ext_type.contains("album"),
                })
            }
            Expression::Ext(ext) => {
                let ext_lower = ext.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::DatabaseSchema(db) => db.ext.to_ascii_lowercase().contains(&ext_lower),
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Model(model) => {
                let model_lower = model.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::DatabaseSchema(db) => {
                        db.exif_vec.get("Model").map_or(false, |model_of_exif| {
                            model_of_exif.to_ascii_lowercase().contains(&model_lower)
                        })
                    }
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Make(make) => {
                let make_lower = make.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::DatabaseSchema(db) => {
                        db.exif_vec.get("Make").map_or(false, |make_of_exif| {
                            make_of_exif.to_ascii_lowercase().contains(&make_lower)
                        })
                    }
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Path(path) => {
                todo!()
            }
            Expression::Album(album_id) => {
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::DatabaseSchema(db) => db.album.contains(&album_id),
                    AbstractData::Album(_) => false,
                })
            }
            Expression::Any(any_identifier) => {
                let any_lower = any_identifier.to_ascii_lowercase();
                Box::new(move |abstract_data: &AbstractData| match abstract_data {
                    AbstractData::DatabaseSchema(db) => {
                        false
                            || db.ext_type.contains(&any_identifier)
                            || db.ext.to_ascii_lowercase().contains(&any_lower)
                            || db.exif_vec.get("Make").map_or(false, |make_of_exif| {
                                make_of_exif.to_ascii_lowercase().contains(&any_lower)
                            })
                            || db.exif_vec.get("Model").map_or(false, |model_of_exif| {
                                model_of_exif.to_ascii_lowercase().contains(&any_lower)
                            })
                            || false
                    }
                    AbstractData::Album(album) => {
                        album.tag.contains(&any_identifier)
                            || "album".to_ascii_lowercase().contains(&any_lower)
                    }
                })
            }
        }
    }
}
