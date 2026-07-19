import type { IsolationId, Row } from '@type/types'
import { defineStore } from 'pinia'

export interface RowGeometrySnapshot {
  readonly rowIndex: number
  readonly topPixelAccumulated: number
  readonly rowHeight: number
  readonly offset: number
}

export const useRowStore = (isolationId: IsolationId) =>
  defineStore('rowStore' + isolationId, {
    state: (): {
      rowData: Map<number, Row> //  Map<rowIndex, Row>
      lastVisibleRow: Map<number, RowGeometrySnapshot>
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
