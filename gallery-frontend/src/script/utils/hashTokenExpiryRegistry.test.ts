import { beforeEach, describe, expect, it } from 'vitest'
import {
  isHashTokenKnownFresh,
  readJwtExpiration,
  registerHashTokenExpiration,
  resetHashTokenExpirations
} from './hashTokenExpiryRegistry'

function tokenWithPayload(payload: object): string {
  const encoded = btoa(JSON.stringify(payload))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '')
  return `header.${encoded}.signature`
}

describe('hash token expiry registry', () => {
  beforeEach(() => {
    resetHashTokenExpirations('mainId')
    resetHashTokenExpirations('subId')
  })

  it('reads finite JWT expirations and rejects malformed payloads', () => {
    expect(readJwtExpiration(tokenWithPayload({ exp: 1234 }))).toBe(1234)
    expect(readJwtExpiration(tokenWithPayload({ exp: '1234' }))).toBeUndefined()
    expect(readJwtExpiration('invalid')).toBeUndefined()
  })

  it('tracks freshness by isolation and preserves the existing expiry boundary', () => {
    registerHashTokenExpiration('mainId', 'hash', 100)

    expect(isHashTokenKnownFresh('mainId', 'hash', 100)).toBe(true)
    expect(isHashTokenKnownFresh('mainId', 'hash', 101)).toBe(false)
    expect(isHashTokenKnownFresh('subId', 'hash', 99)).toBe(false)

    resetHashTokenExpirations('mainId')
    expect(isHashTokenKnownFresh('mainId', 'hash', 99)).toBe(false)
  })

  it('falls back to the checked refresh path when expiration is unknown', () => {
    registerHashTokenExpiration('mainId', 'hash', 200)
    registerHashTokenExpiration('mainId', 'hash', undefined)
    expect(isHashTokenKnownFresh('mainId', 'hash', 100)).toBe(false)
  })
})
