import { IsolationId } from '@/script/common/types'
import axios from 'axios'
import { defineStore } from 'pinia'
import { useWorkerStore } from './workerStore'
import { getToken, storeToken } from '@/indexedDb/token'

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
      async setToken(token: string) {
        this.timestampToken = token

        const workerStore = useWorkerStore(isolationId)
        if (workerStore.imgWorker.length === 0) {
          workerStore.initializeWorker(isolationId)
        }

        await storeToken(token)
        const tokenGetted = await getToken()

        console.log('tokenGetted is', tokenGetted)

        /* this.startAutoRenew()  */
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
          await this.setToken(response.data)
          console.log('Token renewed successfully.')
        } catch (error) {
          console.error('Failed to renew token:', error)
        }
      }
    }
  })()
