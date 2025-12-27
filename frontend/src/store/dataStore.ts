import type { EnrichedUnifiedData, IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useDataStore = (isolationId: IsolationId) =>
  defineStore('DataStore' + isolationId, {
    state: (): {
      data: Map<number, EnrichedUnifiedData> // dataIndex -> data
      hashMapData: Map<string, number> // hash -> dataIndex
      batchFetched: Map<number, boolean> // Tracks the batches of image metadata that have been fetched
    } => ({
      data: new Map(),
      hashMapData: new Map(),
      batchFetched: new Map()
    }),
    actions: {
      // Should be cleared when the layout is changed
      clearAll() {
        this.data.clear()
        this.hashMapData.clear()
        this.batchFetched.clear()
      },
      addTags(index: number, tags: string[]): boolean {
        const data = this.data.get(index)
        if (!data) {
          // Index does not exist
          return false
        }

        // 新結構：data 本身就是 UnifiedData，直接使用 tags 屬性
        tags.forEach((tag) => {
          if (!data.tags.includes(tag)) {
            data.tags.push(tag)
          }
        })
        return true
      },
      removeTags(index: number, tags: string[]): boolean {
        const data = this.data.get(index)
        if (!data) {
          // Index does not exist
          return false
        }

        // 新結構：data 本身就是 UnifiedData，直接使用 tags 屬性
        data.tags = data.tags.filter((tag) => !tags.includes(tag))
        return true
      },
      addAlbums(index: number, albums: string[]): boolean {
        const data = this.data.get(index)
        if (!data) {
          // Index does not exist
          return false
        }

        // 只有 Image/Video 有 albums 屬性
        if (data.type === 'image' || data.type === 'video') {
          albums.forEach((album) => {
            if (!data.albums.includes(album)) {
              data.albums.push(album)
            }
          })
          return true
        }

        // Album 類型不能加入其他 album
        return false
      },
      removeAlbums(index: number, albums: string[]): boolean {
        const data = this.data.get(index)
        if (!data) {
          // Index does not exist
          return false
        }

        // 只有 Image/Video 有 albums 屬性
        if (data.type === 'image' || data.type === 'video') {
          data.albums = data.albums.filter((album) => !albums.includes(album))
          return true
        }

        // Album 類型不能從其他 album 移除
        return false
      }
    }
  })()
