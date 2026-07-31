import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'
import { clearThumbnailElements } from '@/script/utils/thumbnailElementRegistry'
import { markRaw } from 'vue'

export const useImgStore = (isolationId: IsolationId) =>
  defineStore('imgStore' + isolationId, {
    state: (): {
      imgUrl: Map<number, string> // dataIndex -> blobUrl
      imgOriginal: Map<number, string> // dataIndex -> blobUrl
    } => ({
      // Thumbnail DOM updates are handled by the element registry. Keeping this
      // cache raw avoids notifying Vue for every worker completion in a burst.
      imgUrl: markRaw(new Map()),
      imgOriginal: new Map()
    }),
    actions: {
      // Should be cleared when the layout is changed
      clearAll() {
        this.imgUrl.clear()
        this.imgOriginal.clear()
        clearThumbnailElements(isolationId)
      },
      clearForResize() {
        this.imgUrl.clear()
        clearThumbnailElements(isolationId)
      }
    }
  })()
