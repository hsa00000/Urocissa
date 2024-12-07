// functions.ts

import { thumbHashToDataURL } from 'thumbhash'
import { z } from 'zod'
import { DataBaseParse } from './schemas'
import { DataBase, AbstractData, Album } from './types'
import { RouteLocationNormalizedLoaded, Router } from 'vue-router'
import { computed, ComputedRef, inject } from 'vue'
import { useDataStore } from '../../store/dataStore.ts'
import Cookies from 'js-cookie'
import { navBarHeight } from './constants.ts'

/**
 * Creates a DataBase instance from parsed data and timestamp.
 */
export function createDataBase(
  databaseParse: z.infer<typeof DataBaseParse>,
  timestamp: number
): DataBase {
  const database: DataBase = {
    ...databaseParse,
    timestamp: timestamp,
    thumbhashUrl: thumbHashToDataURL(databaseParse.thumbhash),
    filename: databaseParse.alias[0]?.file.split('/').pop() ?? ''
  }
  return database
}

/**
 * Creates an AbstractData instance from DataBase or Album.
 */
export function createAbstractData(data: DataBase | Album): AbstractData {
  if ('hash' in data) {
    return { database: data }
  } else {
    return { album: data }
  }
}

export function useCurrentPage(
  route: RouteLocationNormalizedLoaded
): ComputedRef<'default' | 'all' | 'favorite' | 'archived' | 'trashed' | 'albums' | 'album'> {
  const currentPage = computed(() => {
    if (route.path.startsWith('/favorite')) {
      return 'favorite'
    } else if (route.path.startsWith('/archived')) {
      return 'archived'
    } else if (route.path.startsWith('/trashed')) {
      return 'trashed'
    } else if (route.path.startsWith('/albums')) {
      return 'albums'
    } else if (route.path.startsWith('/album')) {
      return 'album'
    } else if (route.path.startsWith('/all')) {
      return 'all'
    } else {
      return 'default'
    }
  })
  return currentPage
}

export function dater(timestamp: number): string {
  const locale = navigator.language
  return new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric'
  }).format(timestamp)
}

export function getIsolationIdByRoute(route: RouteLocationNormalizedLoaded) {
  const isolationId = route.meta.isReadPage ? 'subId' : 'mainId'
  return isolationId
}

export function getHashIndexDataFromRoute(route: RouteLocationNormalizedLoaded) {
  const isolationId = route.meta.isReadPage ? 'subId' : 'mainId'
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

/**
 * Retrieves the 'jwt' cookie and ensures it exists.
 * @returns The value of the 'jwt' cookie as a string.`
 * @throws {RangeError} If the 'jwt' cookie is not found.
 */
export function getCookiesJwt(): string {
  const jwt = Cookies.get('jwt')
  if (jwt === undefined) {
    throw new RangeError('JWT cookie is missing.')
  }
  return jwt
}

export function getScrollUpperBound(totalHeight: number, windowHeight: number): number {
  return totalHeight - windowHeight + navBarHeight
}

export async function searchByTag(tag: string, router: Router) {
  await router.replace({
    path: '/all',
    query: { search: `tag:${escapeAndWrap(tag)}` }
  })
}

export function escapeAndWrap(str: string): string {
  // 先將字串中的反斜線及雙引號進行轉譯
  const escaped = str
    .replace(/\\/g, '\\\\') // 將 \ 轉為 \\
    .replace(/"/g, '\\"') // 將 " 轉為 \"

  // 將處理後的字串用 " 包起來
  return `"${escaped}"`
}

export function unescapeAndUnwrap(str: string): string {
  // 若字串是以 " 開頭並以 " 結尾，則去除這對引號
  if (str.startsWith('"') && str.endsWith('"')) {
    str = str.slice(1, -1)
  }

  // 將轉譯字元復原
  return str
    .replace(/\\"/g, '"') // 將 \" 還原成 "
    .replace(/\\\\/g, '\\') // 將 \\ 還原成 \
}
