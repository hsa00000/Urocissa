import type { IsolationId } from '@type/types'

const expirationsByIsolation = new Map<IsolationId, Map<string, number>>()

export function readJwtExpiration(token: string): number | undefined {
  const payload = token.split('.')[1]
  if (payload === undefined) {
    return undefined
  }

  try {
    const normalized = payload.replace(/-/g, '+').replace(/_/g, '/')
    const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '=')
    const decoded = JSON.parse(atob(padded)) as { exp?: unknown }
    return typeof decoded.exp === 'number' && Number.isFinite(decoded.exp)
      ? decoded.exp
      : undefined
  } catch {
    return undefined
  }
}

export function registerHashTokenExpiration(
  isolationId: IsolationId,
  hash: string,
  expiration: number | undefined
): void {
  if (expiration === undefined) {
    expirationsByIsolation.get(isolationId)?.delete(hash)
    return
  }

  let expirations = expirationsByIsolation.get(isolationId)
  if (expirations === undefined) {
    expirations = new Map()
    expirationsByIsolation.set(isolationId, expirations)
  }
  expirations.set(hash, expiration)
}

export function isHashTokenKnownFresh(
  isolationId: IsolationId,
  hash: string,
  nowSeconds = Math.floor(Date.now() / 1000)
): boolean {
  const expiration = expirationsByIsolation.get(isolationId)?.get(hash)
  return expiration !== undefined && expiration >= nowSeconds
}

export function resetHashTokenExpirations(isolationId: IsolationId): void {
  expirationsByIsolation.delete(isolationId)
}
