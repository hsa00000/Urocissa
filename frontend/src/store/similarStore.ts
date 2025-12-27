import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'
export const useSimilarStore = (isolationId: IsolationId) =>
  defineStore('similarStore' + isolationId, {
    state: (): {
      usingSimilarMode: boolean
    } => ({
      usingSimilarMode: false
    }),
    actions: {}
  })()
