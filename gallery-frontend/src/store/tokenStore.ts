import { IsolationId, TokenResponse } from '@type/types'
import { jwtDecode } from 'jwt-decode'
import { defineStore } from 'pinia'
import axios from 'axios'
import { TokenResponseSchema } from '@/type/schemas'
import { storeHashToken } from '@/db/db'
import {
  readJwtExpiration,
  registerHashTokenExpiration,
  resetHashTokenExpirations
} from '@/script/utils/hashTokenExpiryRegistry'

interface JwtPayload {
  timestamp: number
  exp?: number
  [key: string]: unknown
}

export const useTokenStore = (isolationId: IsolationId) =>
  defineStore('tokenStore' + isolationId, {
    state: (): {
      timestampToken: string | null
      hashTokenMap: Map<string, string>
      _renewingTimestamp: Promise<void> | null
    } => ({
      timestampToken: null,
      hashTokenMap: new Map<string, string>(),
      _renewingTimestamp: null
    }),

    actions: {
      _isExpired(exp?: number): boolean {
        return typeof exp === 'number' && exp < Math.floor(Date.now() / 1000)
      },

      _decodeToken(token: string): JwtPayload | null {
        try {
          return jwtDecode<JwtPayload>(token)
        } catch (err) {
          console.warn('Invalid JWT:', err)
          return null
        }
      },

      _isTokenExpired(token: string): boolean {
        const decoded = this._decodeToken(token)
        return decoded ? this._isExpired(decoded.exp) : true
      },

      _getTimestampFromToken(): number | null {
        if (this.timestampToken == null) return null
        const decoded = this._decodeToken(this.timestampToken)
        return decoded?.timestamp ?? null
      },

      _getTimestampFromHashToken(hash: string): number | undefined {
        const token = this.hashTokenMap.get(hash)
        if (token === undefined) return undefined
        const decoded = this._decodeToken(token)
        return decoded?.timestamp
      },

      async _updateTimestampToken(): Promise<void> {
        const tokenToRenew = this.timestampToken
        if (tokenToRenew === null) return
        try {
          const response = await axios.post('/post/renew-timestamp-token', {
            token: tokenToRenew
          })
          const parsed: TokenResponse = TokenResponseSchema.parse(response.data)
          if (this.timestampToken === tokenToRenew) this.timestampToken = parsed.token
        } catch (err) {
          console.error('Failed to renew timestamp token:', err)
        }
      },

      async _updateHashToken(expiredToken: string): Promise<string | null> {
        if (this.timestampToken == null) {
          console.error('Missing timestampToken for authorization')
          return null
        }

        try {
          const response = await axios.post(
            '/post/renew-hash-token',
            { expiredHashToken: expiredToken },
            { headers: { Authorization: `Bearer ${this.timestampToken}` } }
          )
          const parsed: TokenResponse = TokenResponseSchema.parse(response.data)
          return parsed.token
        } catch (err) {
          console.error('Failed to update hash token:', err)
          return null
        }
      },

      /**
       * Updates the timestamp token with a lock to prevent concurrent renewals.
       */
      async _refreshTimestampTokenWithLock(): Promise<void> {
        if (this._renewingTimestamp) {
          await this._renewingTimestamp
          return
        }

        this._renewingTimestamp = (async () => {
          await this._updateTimestampToken()
        })().finally(() => {
          this._renewingTimestamp = null
        })

        await this._renewingTimestamp
      },

      async _ensureHashTokenFresh(hash: string): Promise<string | null> {
        const currentToken = this.hashTokenMap.get(hash)
        if (currentToken === undefined) {
          console.error(`No token found for hash: ${hash}`)
          return null
        }

        if (!this._isTokenExpired(currentToken)) {
          return currentToken
        }

        await this.refreshTimestampTokenIfExpired()

        const newToken = await this._updateHashToken(currentToken)
        if (newToken !== null && this.hashTokenMap.get(hash) === currentToken) {
          this.hashTokenMap.set(hash, newToken)
          registerHashTokenExpiration(isolationId, hash, readJwtExpiration(newToken))
        }
        return newToken
      },

      async refreshTimestampTokenIfExpired(): Promise<void> {
        if (this.timestampToken == null || !this._isTokenExpired(this.timestampToken)) return
        await this._refreshTimestampTokenWithLock()
      },

      async refreshHashTokenIfExpired(hash: string): Promise<void> {
        await this._ensureHashTokenFresh(hash)
      },

      async tryRefreshAndStoreTokenToDb(hash: string): Promise<void> {
        const freshToken = await this._ensureHashTokenFresh(hash)
        if (freshToken === null) return

        await storeHashToken(hash, freshToken)
      },

      clearAll(): void {
        this.timestampToken = null
        this.hashTokenMap.clear()
        resetHashTokenExpirations(isolationId)
      }
    }
  })()
