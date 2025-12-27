import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useLocationStore = (isolationId: IsolationId) =>
  defineStore('locateStore' + isolationId, {
    state: (): {
      /**
       * Index of the first photo that appears (partially) in the viewport
       */
      locationIndex: number
      anchor: number | null
    } => ({
      locationIndex: 0,
      anchor: null
    }),
    actions: {
      clearAll() {
        this.locationIndex = 0
        this.anchor = null
      },
      triggerForResize() {
        this.anchor = null
      }
    }
  })()
