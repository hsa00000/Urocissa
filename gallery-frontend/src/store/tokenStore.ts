import { IsolationId, TokenResponse } from '@type/types'
import { jwtDecode } from 'jwt-decode'
import { defineStore } from 'pinia'
import axios from 'axios'
import { TokenResponseSchema } from '@/type/schemas'
import { storeHashToken } from '@/db/db'
import { tryWithMessageStore } from '@/script/utils/try_catch'

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

      _getTimestampFromToken(): number | null {
        if (this.timestampToken == null) return null
        try {
          const decoded = jwtDecode<JwtPayload>(this.timestampToken)
          return typeof decoded.timestamp === 'number' ? decoded.timestamp : null
        } catch (err) {
          console.warn('Invalid JWT:', err)
          return null
        }
      },

      _getTimestampFromHashToken(hash: string): number | undefined {
        const token = this.hashTokenMap.get(hash)
        if (token === undefined) return undefined
        try {
          const decoded = jwtDecode<JwtPayload>(token)
          return typeof decoded.timestamp === 'number' ? decoded.timestamp : undefined
        } catch (err) {
          console.warn(`Invalid JWT for hash: ${hash}`, err)
          return undefined
        }
      },

      async refreshTimestampTokenIfExpired(): Promise<void> {
        const token = this.timestampToken
        if (token == null) return

        let decoded: JwtPayload
        try {
          decoded = jwtDecode<JwtPayload>(token)
          if (!this._isExpired(decoded.exp)) return
        } catch (err) {
          console.warn('Invalid JWT:', err)
          return
        }

        if (this._renewingTimestamp) {
          await this._renewingTimestamp
          return
        }

        this._renewingTimestamp = (async () => {
          const result = await tryWithMessageStore('mainId', async () => {
            const response = await axios.post('/post/renew-timestamp-token', { token })
            const parsed: TokenResponse = TokenResponseSchema.parse(response.data)
            this.timestampToken = parsed.token
            return true
          })

          if (result === undefined) {
            throw new Error('Failed to renew timestamp token')
          }
        })().finally(() => {
          this._renewingTimestamp = null
        })

        await this._renewingTimestamp
      },

      async refreshHashTokenIfExpired(hash: string): Promise<void> {
        const expiredToken = this.hashTokenMap.get(hash)
        if (expiredToken === undefined) {
          throw new Error(`No token found for hash: ${hash}`)
        }

        let decoded: JwtPayload
        try {
          decoded = jwtDecode<JwtPayload>(expiredToken)
        } catch (err) {
          console.warn(`Invalid JWT for hash: ${hash}`, err)
          return
        }

        if (!this._isExpired(decoded.exp)) return

        await tryWithMessageStore('mainId', async () => {
          await this.refreshTimestampTokenIfExpired()

          const timestampToken = this.timestampToken
          if (timestampToken == null) {
            throw new Error('Missing timestampToken for authorization')
          }

          const response = await axios.post(
            '/post/renew-hash-token',
            {
              expiredHashToken: expiredToken
            },
            {
              headers: {
                Authorization: `Bearer ${timestampToken}`
              }
            }
          )

          const parsed: TokenResponse = TokenResponseSchema.parse(response.data)
          this.hashTokenMap.set(hash, parsed.token)
        })
      },

      async tryRefreshAndStoreTokenToDb(hash: string): Promise<boolean> {
        const result = await tryWithMessageStore('mainId', async () => {
          await this.refreshHashTokenIfExpired(hash)
          const token = this.hashTokenMap.get(hash)
          if (token !== undefined) {
            await storeHashToken(hash, token)
            return true
          }
          return false
        })

        return result ?? false
      }
    }
  })()
