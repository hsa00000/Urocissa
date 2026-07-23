use std::collections::{HashMap, HashSet};

use rocket::serde::json::{Error as JsonError, Json};
use serde::Deserialize;

use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::config::{APP_CONFIG, AppConfig};
use crate::public::structure::saved_search::{
    MAX_SAVED_SEARCHES, SavedSearch, SavedSearchContext, normalize_saved_search_name,
    normalize_saved_search_query,
};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSavedSearchRequest {
    name: String,
    context: SavedSearchContext,
    query: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameSavedSearchRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderSavedSearchesRequest {
    ids: Vec<String>,
}

type JsonRequest<'request, T> = Result<Json<T>, JsonError<'request>>;

#[get("/get/saved_searches")]
pub fn get_saved_searches(_auth: GuardAuth) -> AppResult<Json<Vec<SavedSearch>>> {
    let config = APP_CONFIG
        .get()
        .ok_or_else(|| AppError::new(ErrorKind::Internal, "Configuration is not initialized"))?
        .read()
        .map_err(|_| AppError::new(ErrorKind::Internal, "Configuration lock is poisoned"))?;
    Ok(Json(config.private.saved_searches.clone()))
}

#[post("/post/saved_searches", format = "json", data = "<request>")]
pub async fn create_saved_search(
    _auth: GuardAuth,
    read_only: GuardResult<GuardReadOnlyMode>,
    request: JsonRequest<'_, CreateSavedSearchRequest>,
) -> AppResult<Json<Vec<SavedSearch>>> {
    let _ = read_only?;
    let request = parse_json_request(request)?;
    run_mutation(move |searches| create_in(searches, &request)).await
}

#[put("/put/saved_searches/<id>", format = "json", data = "<request>")]
pub async fn rename_saved_search(
    _auth: GuardAuth,
    read_only: GuardResult<GuardReadOnlyMode>,
    id: &str,
    request: JsonRequest<'_, RenameSavedSearchRequest>,
) -> AppResult<Json<Vec<SavedSearch>>> {
    let _ = read_only?;
    let id = id.to_owned();
    let request = parse_json_request(request)?;
    run_mutation(move |searches| rename_in(searches, &id, &request)).await
}

#[put("/put/saved_searches/order", format = "json", data = "<request>")]
pub async fn reorder_saved_searches(
    _auth: GuardAuth,
    read_only: GuardResult<GuardReadOnlyMode>,
    request: JsonRequest<'_, ReorderSavedSearchesRequest>,
) -> AppResult<Json<Vec<SavedSearch>>> {
    let _ = read_only?;
    let request = parse_json_request(request)?;
    run_mutation(move |searches| reorder_in(searches, request)).await
}

#[delete("/delete/saved_searches/<id>")]
pub async fn delete_saved_search(
    _auth: GuardAuth,
    read_only: GuardResult<GuardReadOnlyMode>,
    id: &str,
) -> AppResult<Json<Vec<SavedSearch>>> {
    let _ = read_only?;
    let id = id.to_owned();
    run_mutation(move |searches| delete_in(searches, &id)).await
}

fn parse_json_request<T>(request: JsonRequest<'_, T>) -> AppResult<T> {
    request.map(Json::into_inner).map_err(|error| {
        AppError::new(
            ErrorKind::InvalidInput,
            format!("Invalid saved search request: {error}"),
        )
    })
}

async fn run_mutation(
    mutate: impl FnOnce(&mut Vec<SavedSearch>) -> AppResult<()> + Send + 'static,
) -> AppResult<Json<Vec<SavedSearch>>> {
    tokio::task::spawn_blocking(move || {
        AppConfig::mutate(|config| {
            mutate(&mut config.private.saved_searches)?;
            Ok(config.private.saved_searches.clone())
        })
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Saved search task join error"))?
    .map(Json)
}

fn create_in(searches: &mut Vec<SavedSearch>, request: &CreateSavedSearchRequest) -> AppResult<()> {
    if searches.len() >= MAX_SAVED_SEARCHES {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("Saved search count must not exceed {MAX_SAVED_SEARCHES}"),
        ));
    }

    let name = normalize_saved_search_name(&request.name)
        .map_err(|error| AppError::from_err(ErrorKind::InvalidInput, error))?;
    let query = normalize_saved_search_query(&request.query)
        .map_err(|error| AppError::from_err(ErrorKind::InvalidInput, error))?;
    ensure_unique_name(searches, &name, None)?;
    ensure_unique_target(searches, request.context, &query)?;
    searches.push(SavedSearch::new(name, request.context, query));
    Ok(())
}

fn rename_in(
    searches: &mut [SavedSearch],
    id: &str,
    request: &RenameSavedSearchRequest,
) -> AppResult<()> {
    let name = normalize_saved_search_name(&request.name)
        .map_err(|error| AppError::from_err(ErrorKind::InvalidInput, error))?;
    let index = searches
        .iter()
        .position(|search| search.id == id)
        .ok_or_else(|| AppError::new(ErrorKind::NotFound, "Saved search not found"))?;
    ensure_unique_name(searches, &name, Some(id))?;
    searches[index].name = name;
    Ok(())
}

fn delete_in(searches: &mut Vec<SavedSearch>, id: &str) -> AppResult<()> {
    let original_len = searches.len();
    searches.retain(|search| search.id != id);
    if searches.len() == original_len {
        return Err(AppError::new(ErrorKind::NotFound, "Saved search not found"));
    }
    Ok(())
}

fn reorder_in(
    searches: &mut Vec<SavedSearch>,
    request: ReorderSavedSearchesRequest,
) -> AppResult<()> {
    let requested_ids = request.ids;
    let requested_set = requested_ids.iter().collect::<HashSet<_>>();
    let current_set = searches
        .iter()
        .map(|search| &search.id)
        .collect::<HashSet<_>>();
    if requested_ids.len() != searches.len()
        || requested_set.len() != requested_ids.len()
        || requested_set != current_set
    {
        return Err(AppError::new(
            ErrorKind::Conflict,
            "Saved search order does not match the current list",
        ));
    }

    let mut by_id = searches
        .drain(..)
        .map(|search| (search.id.clone(), search))
        .collect::<HashMap<_, _>>();
    *searches = requested_ids
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect();
    Ok(())
}

fn ensure_unique_name(
    searches: &[SavedSearch],
    name: &str,
    excluded_id: Option<&str>,
) -> AppResult<()> {
    let normalized_name = name.to_lowercase();
    if searches.iter().any(|search| {
        Some(search.id.as_str()) != excluded_id && search.name.to_lowercase() == normalized_name
    }) {
        return Err(AppError::new(
            ErrorKind::Conflict,
            "A saved search with this name already exists",
        ));
    }
    Ok(())
}

fn ensure_unique_target(
    searches: &[SavedSearch],
    context: SavedSearchContext,
    query: &str,
) -> AppResult<()> {
    if searches
        .iter()
        .any(|search| search.context == context && search.query == query)
    {
        return Err(AppError::new(
            ErrorKind::Conflict,
            "An identical saved search already exists",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_request(
        name: &str,
        context: SavedSearchContext,
        query: &str,
    ) -> CreateSavedSearchRequest {
        CreateSavedSearchRequest {
            name: name.to_owned(),
            context,
            query: query.to_owned(),
        }
    }

    #[test]
    fn creates_renames_deletes_and_reorders() {
        let mut searches = Vec::new();
        create_in(
            &mut searches,
            &create_request("Family", SavedSearchContext::Home, "tag:family"),
        )
        .unwrap();
        create_in(
            &mut searches,
            &create_request("Clips", SavedSearchContext::Videos, "tag:trip"),
        )
        .unwrap();
        let first_id = searches[0].id.clone();
        let second_id = searches[1].id.clone();

        rename_in(
            &mut searches,
            &first_id,
            &RenameSavedSearchRequest {
                name: "Family favorites".to_owned(),
            },
        )
        .unwrap();
        reorder_in(
            &mut searches,
            ReorderSavedSearchesRequest {
                ids: vec![second_id.clone(), first_id.clone()],
            },
        )
        .unwrap();
        assert_eq!(searches[0].id, second_id);
        assert_eq!(searches[1].name, "Family favorites");

        delete_in(&mut searches, &first_id).unwrap();
        assert_eq!(searches.len(), 1);
    }

    #[test]
    fn rejects_duplicate_names_and_targets() {
        let mut searches = Vec::new();
        create_in(
            &mut searches,
            &create_request("Family", SavedSearchContext::Home, "tag:family"),
        )
        .unwrap();

        let name_error = create_in(
            &mut searches,
            &create_request("family", SavedSearchContext::Videos, "tag:clips"),
        )
        .unwrap_err();
        assert_eq!(name_error.kind, ErrorKind::Conflict);

        let target_error = create_in(
            &mut searches,
            &create_request("Duplicate", SavedSearchContext::Home, "tag:family"),
        )
        .unwrap_err();
        assert_eq!(target_error.kind, ErrorKind::Conflict);
    }

    #[test]
    fn rejects_non_permutation_order() {
        let mut searches = Vec::new();
        create_in(
            &mut searches,
            &create_request("Family", SavedSearchContext::Home, "tag:family"),
        )
        .unwrap();
        let error = reorder_in(
            &mut searches,
            ReorderSavedSearchesRequest {
                ids: vec!["missing".to_owned()],
            },
        )
        .unwrap_err();
        assert_eq!(error.kind, ErrorKind::Conflict);
    }

    #[test]
    fn rejects_missing_ids_before_other_rename_conflicts() {
        let mut searches = Vec::new();
        create_in(
            &mut searches,
            &create_request("Family", SavedSearchContext::Home, "tag:family"),
        )
        .unwrap();

        let rename_error = rename_in(
            &mut searches,
            "missing",
            &RenameSavedSearchRequest {
                name: "family".to_owned(),
            },
        )
        .unwrap_err();
        assert_eq!(rename_error.kind, ErrorKind::NotFound);

        let delete_error = delete_in(&mut searches, "missing").unwrap_err();
        assert_eq!(delete_error.kind, ErrorKind::NotFound);
    }

    #[test]
    fn enforces_count_and_character_limits() {
        let mut searches = (0..MAX_SAVED_SEARCHES)
            .map(|index| {
                SavedSearch::new(
                    format!("Search {index}"),
                    SavedSearchContext::Home,
                    format!("tag:{index}"),
                )
            })
            .collect::<Vec<_>>();
        let count_error = create_in(
            &mut searches,
            &create_request("One too many", SavedSearchContext::Home, "tag:overflow"),
        )
        .unwrap_err();
        assert_eq!(count_error.kind, ErrorKind::InvalidInput);

        let mut searches = Vec::new();
        let name_error = create_in(
            &mut searches,
            &create_request(
                &"x".repeat(
                    crate::public::structure::saved_search::MAX_SAVED_SEARCH_NAME_CHARS + 1,
                ),
                SavedSearchContext::Home,
                "tag:valid",
            ),
        )
        .unwrap_err();
        assert_eq!(name_error.kind, ErrorKind::InvalidInput);

        let query_error = create_in(
            &mut searches,
            &create_request(
                "Valid",
                SavedSearchContext::Home,
                &"x".repeat(
                    crate::public::structure::saved_search::MAX_SAVED_SEARCH_QUERY_CHARS + 1,
                ),
            ),
        )
        .unwrap_err();
        assert_eq!(query_error.kind, ErrorKind::InvalidInput);
    }

    #[test]
    fn maps_typed_json_errors_to_bad_request_errors() {
        let raw = "{}";
        let parse_error = serde_json::from_str::<CreateSavedSearchRequest>(raw).unwrap_err();
        let error =
            parse_json_request::<CreateSavedSearchRequest>(Err(JsonError::Parse(raw, parse_error)))
                .unwrap_err();

        assert_eq!(error.kind, ErrorKind::InvalidInput);
        assert_eq!(error.http_status(), rocket::http::Status::BadRequest);
    }
}
