import { RouteLocationNormalizedLoaded, Router } from 'vue-router'
import { inject } from 'vue'
import { useDataStore } from '@/store/dataStore'
import { escapeAndWrap } from '@utils/escape'
import { useShareStore } from '@/store/shareStore'

export { getThumbnailSrc } from './thumbnail'

export function getIsolationIdByRoute(route: RouteLocationNormalizedLoaded) {
  const isolationId = route.meta.level === 3 || route.meta.level === 4 ? 'subId' : 'mainId'
  return isolationId
}

export function getHashIndexDataFromRoute(route: RouteLocationNormalizedLoaded) {
  const isolationId = getIsolationIdByRoute(route)
  const storeData = useDataStore(isolationId)

  let hash: string

  if (isolationId === 'mainId' && typeof route.params.hash === 'string') {
    hash = route.params.hash
  } else if (isolationId === 'subId' && typeof route.params.subhash === 'string') {
    hash = route.params.subhash
  } else {
    return undefined
  }

  const index = storeData.hashMapData.get(hash)

  if (index === undefined) {
    return undefined
  }

  const data = storeData.data.get(index)

  if (data === undefined) {
    return undefined
  }

  return { hash: hash, index: index, data: data }
}

export function getArrayValue<T>(array: T[], index: number): T {
  const result = array[index]
  if (result === undefined) {
    throw new RangeError(`Index ${index} is out of bounds for array of length ${array.length}`)
  } else {
    return result
  }
}

/**
 * Retrieves an injected value and ensures it's not undefined.
 * @param key - The injection key.
 * @returns The injected value of type T.
 * @throws {RangeError} If the injected value is undefined.
 */
export function getInjectValue<T>(key: string | symbol): T {
  const result = inject<T>(key)
  if (result === undefined) {
    throw new RangeError(`Injection for key "${String(key)}" is undefined.`)
  }
  return result
}

/**
 * Retrieves a value from a Map and ensures it's not undefined.
 * @param map - The Map to retrieve the value from.
 * @param key - The key whose associated value is to be returned.
 * @returns The value associated with the specified key.
 * @throws {RangeError} If the key does not exist in the Map.
 */
export function getMapValue<K, V>(map: Map<K, V>, key: K): V {
  const value = map.get(key)
  if (value === undefined) {
    throw new RangeError(`No value found for key "${String(key)}" in the map.`)
  }
  return value
}

export function getScrollUpperBound(totalHeight: number, windowHeight: number): number {
  return totalHeight - windowHeight - 4
}

export async function searchByTag(tag: string, router: Router) {
  const { meta, params } = router.currentRoute.value
  const searchQuery = { search: `tag:${escapeAndWrap(tag)}` }

  // if the current baseName is 'share', navigate back to the share root page
  if (meta.baseName === 'share') {
    const albumId = params.albumId as string
    const shareId = params.shareId as string
    await router.push({
      name: 'share',
      params: { albumId, shareId },
      query: searchQuery
    })
  } else {
    await router.push({
      name: 'all',
      query: searchQuery
    })
  }
}

/**
 * Extracts hash from a full URL.
 */
export function extractHashFromAbsoluteUrl(url: URL): string | null {
  const segments = url.pathname.split('/').filter(Boolean)
  const lastSegment = segments.pop()

  return lastSegment?.split('.').shift() ?? null
}

/**
 * Extracts hash from a relative path.
 */
export function extractHashFromPath(path: string): string | null {
  const segments = path.split('/').filter(Boolean)
  const lastSegment = segments.pop()

  return lastSegment?.split('.').shift() ?? null
}

export function getSrc(hash: string, original: boolean, ext: string, updatedAt: number) {
  const compressedOrImported = original ? 'imported' : 'compressed'
  const basePath = `/object/${compressedOrImported}/${hash.slice(0, 2)}/${hash}.${ext}`
  
  return `${basePath}?updated_at=${updatedAt}`
}

export function getSrcOriginal(hash: string, original: boolean, ext: string, updatedAt: number) {
  const shareStore = useShareStore('mainId')
  const baseSrc = getSrc(hash, original, ext, updatedAt)

  if (typeof shareStore.albumId === 'string' && typeof shareStore.shareId === 'string') {
    const separator = baseSrc.includes('?') ? '&' : '?'
    return `${baseSrc}${separator}albumId=${shareStore.albumId}&shareId=${shareStore.shareId}`
  } else {
    return baseSrc
  }
}
