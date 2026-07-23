use std::collections::HashSet;

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_SAVED_SEARCHES: usize = 50;
pub const MAX_SAVED_SEARCH_NAME_CHARS: usize = 80;
pub const MAX_SAVED_SEARCH_QUERY_CHARS: usize = 4_096;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SavedSearchContext {
    Home,
    All,
    Favorite,
    Archived,
    Trashed,
    Albums,
    Videos,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SavedSearch {
    pub id: String,
    pub name: String,
    pub context: SavedSearchContext,
    pub query: String,
}

impl SavedSearch {
    pub fn new(name: String, context: SavedSearchContext, query: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            context,
            query,
        }
    }
}

pub fn normalize_saved_search_name(value: &str) -> Result<String> {
    let value = value.trim();
    ensure!(!value.is_empty(), "Saved search name is required");
    ensure!(
        value.chars().count() <= MAX_SAVED_SEARCH_NAME_CHARS,
        "Saved search name must not exceed {MAX_SAVED_SEARCH_NAME_CHARS} characters"
    );
    Ok(value.to_owned())
}

pub fn normalize_saved_search_query(value: &str) -> Result<String> {
    let value = value.trim();
    ensure!(!value.is_empty(), "Saved search query is required");
    ensure!(
        value.chars().count() <= MAX_SAVED_SEARCH_QUERY_CHARS,
        "Saved search query must not exceed {MAX_SAVED_SEARCH_QUERY_CHARS} characters"
    );
    Ok(value.to_owned())
}

pub fn normalize_and_validate_saved_searches(searches: &mut Vec<SavedSearch>) -> Result<()> {
    ensure!(
        searches.len() <= MAX_SAVED_SEARCHES,
        "Saved search count must not exceed {MAX_SAVED_SEARCHES}"
    );

    let mut names = HashSet::with_capacity(searches.len());
    let mut targets = HashSet::with_capacity(searches.len());
    let mut ids = HashSet::with_capacity(searches.len());

    for search in searches {
        search.name = normalize_saved_search_name(&search.name)?;
        search.query = normalize_saved_search_query(&search.query)?;
        ensure!(
            Uuid::parse_str(&search.id).is_ok(),
            "Saved search ID is not a valid UUID"
        );
        ensure!(
            ids.insert(search.id.clone()),
            "Saved search IDs must be unique"
        );
        ensure!(
            names.insert(search.name.to_lowercase()),
            "Saved search names must be unique"
        );
        ensure!(
            targets.insert((search.context, search.query.clone())),
            "Saved search targets must be unique"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_names_and_queries() {
        assert_eq!(normalize_saved_search_name("  Family  ").unwrap(), "Family");
        assert_eq!(
            normalize_saved_search_query("  tag:family  ").unwrap(),
            "tag:family"
        );
    }

    #[test]
    fn validates_collection_uniqueness() {
        let first = SavedSearch::new(
            "Family".to_owned(),
            SavedSearchContext::Home,
            "tag:family".to_owned(),
        );
        let mut duplicate_name = vec![
            first,
            SavedSearch::new(
                "family".to_owned(),
                SavedSearchContext::Videos,
                "tag:video".to_owned(),
            ),
        ];

        assert!(normalize_and_validate_saved_searches(&mut duplicate_name).is_err());
    }

    #[test]
    fn context_serializes_as_route_name() {
        assert_eq!(
            serde_json::to_value(SavedSearchContext::Favorite).unwrap(),
            serde_json::json!("favorite")
        );
    }
}
