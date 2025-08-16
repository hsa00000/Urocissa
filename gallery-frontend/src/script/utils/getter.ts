import { RouteLocationNormalizedLoaded, Router } from 'vue-router'
import { inject } from 'vue'
import { useDataStore } from '@/store/dataStore'
import { escapeAndWrap } from '@utils/escape'
import { useShareStore } from '@/store/shareStore'

export function getIsolationIdByRoute(route: RouteLocationNormalizedLoaded) {
  const isolationId = route.meta.level === 4 ? 'subId' : 'mainId'
  return isolationId
}

export function getHashIndexDataFromRoute(route: RouteLocationNormalizedLoaded) {
  const isolationId = route.meta.level === 4 ? 'subId' : 'mainId'
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

  switch (tag) {
    case '_favorite':
      await router.push({ name: 'favorite' })
      return
    case '_archived':
      await router.push({ name: 'archived' })
      return
    case '_trashed':
      await router.push({ name: 'trashed' })
      return
    default:
      // if the current baseName is 'share', navigate back to the share root page ──
      if (meta.baseName === 'share') {
        const albumId = params.albumId as string
        const shareId = params.shareId as string
        await router.push({
          name: 'share',
          params: { albumId, shareId }, // Reconstruct /share/:albumId-:shareId
          query: searchQuery
        })
      } else {
        await router.push({
          name: 'all',
          query: searchQuery
        })
      }
  }
}

// 適用於完整 URL（包含 http:// 或 https://）
export function extractHashFromAbsoluteUrl(url: URL): string | null {
  const segments = url.pathname.split('/').filter(Boolean) // 移除空字串
  const lastSegment = segments.pop() // 取得最後一個片段

  return lastSegment?.split('.').shift() ?? null // 移除副檔名，保留 hash
}

// 適用於相對路徑（不含 http:// 或 https://）
export function extractHashFromPath(path: string): string | null {
  const segments = path.split('/').filter(Boolean) // 移除空字串
  const lastSegment = segments.pop() // 取得最後一個片段

  return lastSegment?.split('.').shift() ?? null // 移除副檔名，保留 hash
}

export function getSrc(hash: string, original: boolean, ext: string) {
  const compressedOrImported = original ? 'imported' : 'compressed'
  return `/object/${compressedOrImported}/${hash.slice(0, 2)}/${hash}.${ext}`
}

export function getSrcOriginal(hash: string, original: boolean, ext: string) {
  const shareStore = useShareStore('mainId')
  const compressedOrImported = original ? 'imported' : 'compressed'
  const basePath = `/object/${compressedOrImported}/${hash.slice(0, 2)}/${hash}.${ext}`

  if (typeof shareStore.albumId === 'string' && typeof shareStore.shareId === 'string') {
    return `${basePath}?albumId=${shareStore.albumId}&shareId=${shareStore.shareId}`
  } else {
    return basePath
  }
}
