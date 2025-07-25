import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useConstStore = (isolationId: IsolationId) =>
  defineStore('constStore' + isolationId, {
    state: (): {
      disableImg: boolean
      subRowHeightScale: number
    } => ({
      disableImg: false,
      subRowHeightScale: 1.0
    }),
    actions: {}
  })()
