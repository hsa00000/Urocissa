// functions.ts

import { thumbHashToDataURL } from 'thumbhash'
import { z } from 'zod'
import { AlbumParse, DataBaseParse } from './schemas'
import { Database, AbstractData, Album } from './types'
import { RouteLocationNormalizedLoaded, Router } from 'vue-router'
import { inject } from 'vue'
import { useDataStore } from '../../store/dataStore.ts'
import Cookies from 'js-cookie'
import { navBarHeight } from './constants.ts'

/**
 * Creates a Database instance from parsed data and timestamp.
 */
export function createDataBase(
  databaseParse: z.infer<typeof DataBaseParse>,
  timestamp: number
): Database {
  const database: Database = {
    ...databaseParse,
    timestamp: timestamp,
    thumbhashUrl: thumbHashToDataURL(databaseParse.thumbhash),
    filename: databaseParse.alias[0]?.file.split('/').pop() ?? ''
  }
  return database
}

export function createAlbum(albumParse: z.infer<typeof AlbumParse>, timestamp: number): Album {
  const album: Album = {
    ...albumParse,
    timestamp: timestamp,
    thumbhashUrl: albumParse.thumbhash ? thumbHashToDataURL(albumParse.thumbhash) : null
  }
  return album
}

/**
 * Creates an AbstractData instance from Database or Album.
 */
export function createAbstractData(data: Database | Album): AbstractData {
  if ('hash' in data) {
    return { database: data }
  } else {
    return { album: data }
  }
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
  switch (tag) {
    case '_favorite':
      await router.push({ path: '/favorite' })
      break
    case '_archived':
      await router.push({ path: '/archived' })
      break
    case '_trashed':
      await router.push({ path: '/trashed' })
      break
    default:
      await router.push({
        path: '/all',
        query: { search: `tag:${escapeAndWrap(tag)}` }
      })
      break
  }
}

export function escapeAndWrap(str: string): string {
  // First, escape backslashes and double quotes in the string
  const escaped = str
    .replace(/\\/g, '\\\\') // Convert \ to \\
    .replace(/"/g, '\\"') // Convert " to \"

  // Wrap the processed string in double quotes
  return `"${escaped}"`
}

export function unescapeAndUnwrap(str: string): string {
  // If the string starts and ends with double quotes, remove them
  if (str.startsWith('"') && str.endsWith('"')) {
    str = str.slice(1, -1)
  }

  // Restore escaped characters
  return str
    .replace(/\\"/g, '"') // Convert \" back to "
    .replace(/\\\\/g, '\\') // Convert \\ back to \
}

export function formatDuration(durationString: string) {
  // Convert the duration string to a number and truncate to the integer part
  const durationInSeconds = Math.floor(parseFloat(durationString))

  // Calculate hours, minutes, and seconds
  const hours = Math.floor(durationInSeconds / 3600)
  const minutes = Math.floor((durationInSeconds % 3600) / 60)
  const seconds = durationInSeconds % 60

  // Determine the formatted duration based on the presence of hours, minutes, and seconds
  let formattedDuration = ''
  if (hours > 0) {
    formattedDuration = `${hours}:${minutes.toString().padStart(2, '0')}:${seconds
      .toString()
      .padStart(2, '0')}`
  } else {
    formattedDuration = `${minutes.toString().padStart(2, '0')}:${seconds
      .toString()
      .padStart(2, '0')}`
  }

  return formattedDuration
}
