import { IsolationId, ScrollbarData } from '@type/types'
import { defineStore } from 'pinia'
export const useScrollbarStore = (isolationId: IsolationId) =>
  defineStore('scrollbarStore' + isolationId, {
    state: (): {
      scrollbarDataArray: ScrollbarData[]
      scrollbarDataArrayYear: ScrollbarData[]
      initialized: boolean
      isDragging: boolean
      isHovering: boolean
    } => ({
      scrollbarDataArray: [],
      scrollbarDataArrayYear: [],
      initialized: false,
      isDragging: false,
      isHovering: false
    }),
    actions: {
      initialize(scrollbarDataArray: ScrollbarData[]) {
        this.scrollbarDataArray = scrollbarDataArray
        this.scrollbarDataArrayYear = []
        let year: number | null = null
        this.scrollbarDataArray.forEach((scrollbarData) => {
          if (year !== scrollbarData.year) {
            year = scrollbarData.year
            this.scrollbarDataArrayYear.push(scrollbarData)
          }
        })

        this.initialized = true
      }
    }
  })()
