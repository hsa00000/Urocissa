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
        const data = this.data.get(index)
        if (!data) {
          // Index does not exist
          return false
        }

        // Determine the target object: database or album
        const target = data.database ?? data.album

        if (target) {
          tags.forEach((tag) => {
            if (!target.tags.includes(tag)) {
              target.tags.push(tag)
            }
          })
          return true
        }

        // Neither database nor album exists for this index
        throw new Error(`No database or album found for index ${index}`)
      },
      removeTags(index: number, tags: string[]): boolean {
        const data = this.data.get(index)
        if (!data) {
          // Index does not exist
          return false
        }

        // Determine the target object: database or album
        const target = data.database ?? data.album

        if (target) {
          target.tags = target.tags.filter((tag) => !tags.includes(tag))
          return true
        }

        // Neither database nor album exists for this index
        throw new Error(`No database or album found for index ${index}`)
      },
      addAlbums(index: number, albums: string[]): boolean {
        const data = this.data.get(index)
        if (!data) {
          // Index does not exist
          return false
        }

        // Determine the target object: database or album
        const target = data.database

        if (target) {
          albums.forEach((album) => {
            if (!target.album.includes(album)) {
              target.album.push(album)
            }
          })
          return true
        }

        if (data.album) {
          return false
        }

        // Neither database nor album exists for this index
        throw new Error(`No database found for index ${index}`)
      },
      removeAlbums(index: number, albums: string[]): boolean {
        const data = this.data.get(index)
        if (!data) {
          // Index does not exist
          return false
        }

        // Determine the target object: database or album
        const target = data.database

        if (target) {
          target.album = target.album.filter((album) => !albums.includes(album))
          return true
        }

        if (data.album) {
          return false
        }

        // Neither database nor album exists for this index
        throw new Error(`No database found for index ${index}`)
      }
    }
  })()
