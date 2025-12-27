import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useOffsetStore = (isolationId: IsolationId) =>
  defineStore('offsetStore' + isolationId, {
    state: (): {
      offset: Map<number, number> // Map<rowIndex, offset>
      accumulatedAll: number
    } => ({
      offset: new Map(),
      accumulatedAll: 0
    }),
    actions: {
      accumulatedOffset(currentRowIndex: number): number {
        let sum = 0
        this.offset.forEach((value, key) => {
          if (key < currentRowIndex) {
            sum += value
          }
          setTimeout(() => ({}), 0)
        })
        return sum
      },
      clearAll() {
        this.offset.clear()
        this.accumulatedAll = 0
      }
    }
  })()
