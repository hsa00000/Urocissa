import type { LocationQuery, LocationQueryValue } from 'vue-router'

interface RootRouteKeyInput {
  baseName: unknown
  level: unknown
  query: LocationQuery
  concurrencyNumber: number
  homeKey: boolean
}

function queryValue(
  value: LocationQueryValue | LocationQueryValue[] | undefined
): string | LocationQueryValue[] {
  if (typeof value === 'string') return value
  if (Array.isArray(value)) return [...value]
  return ''
}

/**
 * The root route layer owns Level 1. Reader-only query changes must not replace
 * it, otherwise every nested overlay is destroyed and reactivated.
 */
export function createRootRouteKey({
  baseName,
  level,
  query,
  concurrencyNumber,
  homeKey
}: RootRouteKeyInput): string {
  const isReaderRoute = typeof level === 'number' && level >= 3
  const search = queryValue(query.search)
  const locate = isReaderRoute ? '' : queryValue(query.locate)
  const priorityId = isReaderRoute ? '' : queryValue(query.priority_id)
  const sort = isReaderRoute ? '' : queryValue(query.sort)

  // A structured key avoids delimiter collisions (including array values such
  // as ["a,b"] versus ["a", "b"]) that could otherwise keep a stale root
  // collection mounted after a real query change.
  return JSON.stringify([
    String(baseName),
    search,
    locate,
    priorityId,
    sort,
    concurrencyNumber,
    homeKey
  ])
}
