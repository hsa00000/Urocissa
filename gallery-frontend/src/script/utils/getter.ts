import { RouteLocationNormalizedLoaded, Router } from 'vue-router'
import { inject } from 'vue'
import { useDataStore } from '@/store/dataStore'
import Cookies from 'js-cookie'
import { escapeAndWrap } from '@utils/escape'
import { navBarHeight } from '../common/constants'
import { getSrc } from '@/../config'

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

export function getSrcWithToken(
  hash: string,
  original: boolean,
  ext: string,
  _password: string,
  _customParams: unknown,
  token: string
) {
  const url = getSrc(hash, original, ext, _password, _customParams)
  const urlWithToken = `${url}?token=${token}`
  console.log('urlWithToken is', urlWithToken)

  return urlWithToken
}
