use super::Expression;
use rusqlite::ToSql;

impl Expression {
    pub fn to_sql(&self, hide_metadata: bool, shared_album_id: Option<&str>) -> (String, Vec<Box<dyn ToSql + Send + Sync>>) {
        match self {
            Expression::And(exprs) => {
                if exprs.is_empty() {
                    return ("1=1".to_string(), vec![]);
                }
                let mut sql_parts = Vec::new();
                let mut params: Vec<Box<dyn ToSql + Send + Sync>> = Vec::new();
                for expr in exprs {
                    let (sql, p) = expr.to_sql(hide_metadata, shared_album_id);
                    sql_parts.push(format!("({})", sql));
                    params.extend(p);
                }
                (sql_parts.join(" AND "), params)
            }
            Expression::Or(exprs) => {
                if exprs.is_empty() {
                    return ("1=0".to_string(), vec![]);
                }
                let mut sql_parts = Vec::new();
                let mut params: Vec<Box<dyn ToSql + Send + Sync>> = Vec::new();
                for expr in exprs {
                    let (sql, p) = expr.to_sql(hide_metadata, shared_album_id);
                    sql_parts.push(format!("({})", sql));
                    params.extend(p);
                }
                (sql_parts.join(" OR "), params)
            }
            Expression::Not(expr) => {
                let (sql, params) = expr.to_sql(hide_metadata, shared_album_id);
                (format!("NOT ({})", sql), params)
            }
            Expression::Tag(tag) => {
                if hide_metadata {
                    ("1=0".to_string(), vec![])
                } else {
                    ("EXISTS (SELECT 1 FROM object_tags WHERE object_tags.object_id = objects.id AND object_tags.tag = ?)".to_string(), vec![Box::new(tag.clone())])
                }
            },
            Expression::ExtType(ext_type) => (
                "ext_type = ?".to_string(),
                vec![Box::new(ext_type.clone())],
            ),
            Expression::Ext(ext) => (
                "ext = ?".to_string(),
                vec![Box::new(ext.clone())],
            ),
            Expression::Album(album_id) => {
                if hide_metadata {
                    if let Some(shared_id) = shared_album_id {
                        if album_id.as_str() == shared_id {
                             ("EXISTS (SELECT 1 FROM album_objects WHERE album_objects.object_id = objects.id AND album_objects.album_id = ?)".to_string(), vec![Box::new(album_id.to_string())])
                        } else {
                            ("1=0".to_string(), vec![])
                        }
                    } else {
                        ("1=0".to_string(), vec![])
                    }
                } else {
                    ("EXISTS (SELECT 1 FROM album_objects WHERE album_objects.object_id = objects.id AND album_objects.album_id = ?)".to_string(), vec![Box::new(album_id.to_string())])
                }
            },
            Expression::Model(model) => (
                "json_extract(exif, '$.Model') = ?".to_string(),
                vec![Box::new(model.clone())],
            ),
            Expression::Make(make) => (
                "json_extract(exif, '$.Make') = ?".to_string(),
                vec![Box::new(make.clone())],
            ),
            Expression::Path(path) => {
                if hide_metadata {
                    ("1=0".to_string(), vec![])
                } else {
                    ("EXISTS (SELECT 1 FROM json_each(alias) WHERE json_extract(value, '$.path') LIKE ?)".to_string(), vec![Box::new(format!("%{}%", path))])
                }
            },
            Expression::Any(query) => {
                if hide_metadata {
                     ("(ext LIKE ? OR ext_type LIKE ?)".to_string(),
                    vec![
                        Box::new(format!("%{}%", query)),
                        Box::new(format!("%{}%", query)),
                    ])
                } else {
                    ("(ext LIKE ? OR ext_type LIKE ? OR EXISTS (SELECT 1 FROM object_tags WHERE object_tags.object_id = objects.id AND object_tags.tag LIKE ?))".to_string(),
                    vec![
                        Box::new(format!("%{}%", query)),
                        Box::new(format!("%{}%", query)),
                        Box::new(format!("%{}%", query)),
                    ])
                }
            },
        }
    }
}
