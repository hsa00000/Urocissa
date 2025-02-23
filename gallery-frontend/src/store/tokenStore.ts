import { IsolationId } from '@/script/common/types'
import axios from 'axios'
import { defineStore } from 'pinia'
export const useTokenStore = (isolationId: IsolationId) =>
  defineStore('tokenStore' + isolationId, {
    state: (): {
      timestampToken: string | null
      tokenRenewTimeout: ReturnType<typeof setTimeout> | null
    } => ({
      timestampToken: null,
      tokenRenewTimeout: null
    }),
    actions: {
      setToken(token: string) {
        this.timestampToken = token
        this.startAutoRenew()
      },
      startAutoRenew() {
        if (this.tokenRenewTimeout) {
          clearTimeout(this.tokenRenewTimeout)
        }
        this.tokenRenewTimeout = setTimeout(() => {
          this.renewToken().catch((error: unknown) => {
            console.error('Token renewal failed in timer:', error)
          })
        }, 5000)
      },

      async renewToken() {
        console.log('use token', this.timestampToken)

        try {
          const response = await axios.post<string>(
            '/post/renew-timestamp-token',
            {
              token: this.timestampToken
            },
            { headers: { 'Content-Type': 'application/json' } }
          )
          this.setToken(response.data)
          console.log('Token renewed successfully.')
        } catch (error) {
          console.error('Failed to renew token:', error)
        }
      }
    }
  })()
