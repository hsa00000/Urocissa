import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useImgStore = (isolationId: IsolationId) =>
  defineStore('imgStore' + isolationId, {
    state: (): {
      imgUrl: Map<number, string> // dataIndex -> blobUrl
      imgOriginal: Map<number, string> // dataIndex -> blobUrl
    } => ({
      imgUrl: new Map(),
      imgOriginal: new Map()
    }),
    actions: {
      // Should be cleared when the layout is changed
      clearAll() {
        this.imgUrl.clear()
        this.imgOriginal.clear()
      },
      clearForResize() {
        this.imgUrl.clear()
      }
    }
  })()
