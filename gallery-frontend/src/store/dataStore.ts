import type { AbstractData, IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useDataStore = (isolationId: IsolationId) =>
  defineStore('DataStore' + isolationId, {
    state: (): {
      data: Map<number, AbstractData> // dataIndex -> data
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
        const wrapped = this.data.get(index)
        if (!wrapped) return false

        const target = wrapped.data
        tags.forEach((tag) => {
          if (!target.tags.includes(tag)) {
            target.tags.push(tag)
          }
        })
        return true
      },
      removeTags(index: number, tags: string[]): boolean {
        const wrapped = this.data.get(index)
        if (!wrapped) return false

        wrapped.data.tags = wrapped.data.tags.filter((tag) => !tags.includes(tag))
        return true
      },
      addAlbums(index: number, albums: string[]): boolean {
        const wrapped = this.data.get(index)
        if (!wrapped) return false

        const target = wrapped.data
        if (target.type === 'album') return false

        albums.forEach((album) => {
          if (!target.albums.includes(album)) {
            target.albums.push(album)
          }
        })
        return true
      },
      removeAlbums(index: number, albums: string[]): boolean {
        const wrapped = this.data.get(index)
        if (!wrapped) return false

        const target = wrapped.data
        if (target.type === 'album') return false

        target.albums = target.albums.filter((album) => !albums.includes(album))
        return true
      }
    }
  })()
