import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useQueueStore = (isolationId: IsolationId) =>
  defineStore('queueStore' + isolationId, {
    state: (): {
      // Set to keep track of image IDs that have been fetched and will be sent to canva
      img: Set<number>
      original: Set<number>
      row: Set<number>
    } => ({
      img: new Set(),
      original: new Set(),
      row: new Set()
    }),
    actions: {
      // Clears the set of image IDs
      // Should be used whenever the layout is changed
      clearAll() {
        this.img.clear()
        this.original.clear()
        this.row.clear()
      }
    }
  })()
