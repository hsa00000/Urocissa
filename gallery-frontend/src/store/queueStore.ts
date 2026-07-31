import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'
import { markRaw } from 'vue'

export const useQueueStore = (isolationId: IsolationId) =>
  defineStore('queueStore' + isolationId, {
    state: (): {
      // Set to keep track of image IDs that have been fetched and will be sent to canva
      img: Set<number>
      original: Set<number>
      row: Set<number>
    } => ({
      // These sets are imperative request-deduplication state. No rendered
      // view consumes their mutations, so Vue proxy notifications are wasted.
      img: markRaw(new Set<number>()),
      original: markRaw(new Set<number>()),
      row: markRaw(new Set<number>())
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
