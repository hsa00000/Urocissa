import { IsolationId, TokenResponse } from '@type/types'
import { jwtDecode } from 'jwt-decode'
import { defineStore } from 'pinia'
import axios from 'axios'
import { TokenResponseSchema } from '@/type/schemas'
import { storeHashToken } from '@/db/db'

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
      // === 基礎工具方法 ===
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

      // === Token 信息獲取 ===
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

      // === 核心更新邏輯 ===
      async _updateTimestampToken(): Promise<void> {
        const response = await axios.post('/post/renew-timestamp-token', {
          token: this.timestampToken
        })
        const parsed: TokenResponse = TokenResponseSchema.parse(response.data)
        this.timestampToken = parsed.token
      },

      async _updateHashToken(expiredToken: string): Promise<string> {
        if (this.timestampToken == null) {
          throw new Error('Missing timestampToken for authorization')
        }

        const response = await axios.post(
          '/post/renew-hash-token',
          { expiredHashToken: expiredToken },
          { headers: { Authorization: `Bearer ${this.timestampToken}` } }
        )

        const parsed: TokenResponse = TokenResponseSchema.parse(response.data)
        return parsed.token
      },

      // === 帶併發控制的 Timestamp Token 更新 ===
      async _refreshTimestampTokenWithLock(): Promise<void> {
        if (this._renewingTimestamp) {
          await this._renewingTimestamp
          return
        }

        this._renewingTimestamp = (async () => {
          try {
            await this._updateTimestampToken()
          } catch (err) {
            console.error('Failed to renew timestamp token:', err)
            throw err
          } finally {
            this._renewingTimestamp = null
          }
        })()

        await this._renewingTimestamp
      },

      // === 通用的 Hash Token 處理邏輯 ===
      async _ensureHashTokenFresh(hash: string): Promise<string> {
        const currentToken = this.hashTokenMap.get(hash)
        if (currentToken === undefined) {
          throw new Error(`No token found for hash: ${hash}`)
        }

        if (!this._isTokenExpired(currentToken)) {
          return currentToken
        }

        // 確保 timestamp token 最新 (與原始代碼行為一致)
        await this.refreshTimestampTokenIfExpired()

        if (this.timestampToken == null) {
          throw new Error('Missing timestampToken for authorization')
        }

        // 更新 hash token
        const newToken = await this._updateHashToken(currentToken)
        this.hashTokenMap.set(hash, newToken)
        return newToken
      },

      // === 公開接口 ===
      async refreshTimestampTokenIfExpired(): Promise<void> {
        if (this.timestampToken == null || !this._isTokenExpired(this.timestampToken)) return
        await this._refreshTimestampTokenWithLock()
      },

      async refreshHashTokenIfExpired(hash: string): Promise<void> {
        try {
          await this._ensureHashTokenFresh(hash)
        } catch (err) {
          console.error(`Failed to renew token for hash: ${hash}`, err)
          // 原始代碼會吞掉錯誤，所以這裡不重新拋出
        }
      },

      async tryRefreshAndStoreTokenToDb(hash: string): Promise<boolean> {
        try {
          const freshToken = await this._ensureHashTokenFresh(hash)
          await storeHashToken(hash, freshToken)
          return true
        } catch (err) {
          console.error(`Failed to refresh and store token for hash: ${hash}`, err)
          return false
        }
      }
    }
  })()
