import { IsolationId } from '@/script/common/types'
import { defineStore } from 'pinia'
export const useTokenStore = (isolationId: IsolationId) =>
  defineStore('tokenStore' + isolationId, {
    state: (): {
      timestampToken: string | null
    } => ({
      timestampToken: null
    }),
    actions: {}
  })()
