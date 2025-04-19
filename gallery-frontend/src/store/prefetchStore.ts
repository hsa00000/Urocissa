import { fixedBigRowHeight, layoutBatchNumber } from '@/type/constants'
import { IsolationId, Share } from '@type/types'
import { defineStore } from 'pinia'

export const usePrefetchStore = (isolationId: IsolationId) =>
  defineStore('prefetchStore' + isolationId, {
    state: (): {
      windowWidth: number
      timestamp: number | null
      totalHeight: number
      totalHeightOriginal: number
      dataLength: number // length of all photos
      rowLength: number // length of all photo batches
      locateTo: number | null
      share: null | Share
      updateVisibleRowTrigger: boolean
      updateFetchRowTrigger: boolean
    } => ({
      windowWidth: 0,
      timestamp: null,
      totalHeight: 0,
      totalHeightOriginal: 0,
      dataLength: 0,
      rowLength: 0,
      locateTo: null,
      share: null,
      updateVisibleRowTrigger: false,
      updateFetchRowTrigger: false
    }),
    actions: {
      calculateLength(dataLength: number) {
        this.dataLength = dataLength
        this.rowLength = Math.ceil(dataLength / layoutBatchNumber)
        this.totalHeight = Math.ceil(dataLength / layoutBatchNumber) * fixedBigRowHeight
        this.totalHeightOriginal = this.totalHeight
      },
      clearAll() {
        this.timestamp = null
        this.totalHeight = 0
        this.totalHeightOriginal = 0
        this.dataLength = 0
        this.locateTo = null
        this.updateVisibleRowTrigger = !this.updateVisibleRowTrigger
      },
      clearForResize() {
        this.totalHeight = Math.ceil(this.dataLength / layoutBatchNumber) * fixedBigRowHeight
        this.totalHeightOriginal = this.totalHeight
        this.updateVisibleRowTrigger = !this.updateVisibleRowTrigger
      }
    }
  })()
