import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useConstStore = (isolationId: IsolationId) =>
  defineStore('constStore' + isolationId, {
    state: (): {
      disableImg: boolean
      subRowHeightScale: number
    } => ({
      disableImg: false,
      subRowHeightScale: 250
    }),
    actions: {
      incrementSubRowHeightScale() {
        this.subRowHeightScale = Math.min(350, this.subRowHeightScale + 50)
      },
      decrementSubRowHeightScale() {
        this.subRowHeightScale = Math.max(150, this.subRowHeightScale - 50)
      }
    }
  })()
