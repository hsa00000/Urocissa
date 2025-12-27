import type { IsolationId, Row } from '@type/types'
import { defineStore } from 'pinia'

export const useRowStore = (isolationId: IsolationId) =>
  defineStore('rowStore' + isolationId, {
    state: (): {
      rowData: Map<number, Row> //  Map<rowIndex, Row>
      lastVisibleRow: Map<number, Row>
      firstRowFetched: boolean
    } => ({
      rowData: new Map(),
      lastVisibleRow: new Map(),
      firstRowFetched: false // prevent BufferPlaceholder showing when first row has not been fetched
    }),
    actions: {
      clearAll() {
        this.rowData.clear()
        this.lastVisibleRow.clear()
        this.firstRowFetched = false
      },
      clearForResize() {
        this.rowData.clear()
        this.lastVisibleRow.clear()
      }
    }
  })()
