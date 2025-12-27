import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useInitializedStore = (isolationId: IsolationId) =>
  defineStore('initializedStore' + isolationId, {
    state: (): {
      initialized: boolean
    } => ({
      initialized: false
    }),
    actions: {}
  })()
