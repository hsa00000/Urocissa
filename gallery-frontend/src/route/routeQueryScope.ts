import type { LocationQuery } from 'vue-router'

/**
 * Reader collection controls belong to Level 3. They must remain on the reader
 * history entry, but must not leak into its Level 2 or Level 1 ancestors.
 */
export function withoutReaderOnlyQuery(query: LocationQuery): LocationQuery {
  const parentQuery: LocationQuery = { ...query }
  delete parentQuery.subSearch
  delete parentQuery.sort
  delete parentQuery.locate
  delete parentQuery.priority_id
  return parentQuery
}
