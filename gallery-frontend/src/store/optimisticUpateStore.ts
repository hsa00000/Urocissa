import { AbstractData, IsolationId } from '@type/types'
import { defineStore } from 'pinia'
import { useDataStore } from './dataStore'

export interface EditTagsPayload {
  indexSet: Set<number>
  addTagsArray: string[]
  removeTagsArray: string[]
  timestamp: number
}

export interface EditAlbumsPayload {
  indexSet: Set<number>
  addAlbumsArray: string[]
  removeAlbumsArray: string[]
  timestamp: number
}

export interface EditStatusPayload {
  indexArray: number[]
  timestamp: number
  isFavorite?: boolean
  isArchived?: boolean
  isTrashed?: boolean
}

export const useOptimisticStore = (isolationId: IsolationId) =>
  defineStore('optimisticUpdateStore' + isolationId, {
    state: (): {
      backupData: Map<number, AbstractData> // dataIndex -> data
      queueTagsUpdate: EditTagsPayload[]
      queueAlbumsUpdate: EditAlbumsPayload[]
      queueStatusUpdate: EditStatusPayload[]
    } => ({
      backupData: new Map(),
      queueTagsUpdate: [],
      queueAlbumsUpdate: [],
      queueStatusUpdate: []
    }),
    actions: {
      clearAll() {
        this.backupData.clear()
        this.queueTagsUpdate = []
        this.queueAlbumsUpdate = []
        this.queueStatusUpdate = []
      },
      optimisticUpdateTags(payload: EditTagsPayload, pushIntoQueue: boolean) {
        const dataStore = useDataStore(isolationId)
        for (const index of dataStore.data.keys()) {
          if (payload.indexSet.has(index)) {
            const addTagsResult = dataStore.addTags(index, payload.addTagsArray)

            const removeTagsResult = dataStore.removeTags(index, payload.removeTagsArray)
            if (addTagsResult && removeTagsResult) {
              payload.indexSet.delete(index)
            }
          }
        }

        if (
          pushIntoQueue && // only the new task should be pushed
          payload.indexSet.size !== 0
        ) {
          // some data has not been fetched yet
          this.queueTagsUpdate.push(payload)
        }
      },
      optimisticUpdateAlbums(payload: EditAlbumsPayload, pushIntoQueue: boolean) {
        const dataStore = useDataStore(isolationId)
        for (const index of dataStore.data.keys()) {
          if (payload.indexSet.has(index)) {
            const addTagsResult = dataStore.addAlbums(index, payload.addAlbumsArray)

            const removeTagsResult = dataStore.removeAlbums(index, payload.removeAlbumsArray)
            if (addTagsResult && removeTagsResult) {
              payload.indexSet.delete(index)
            }
          }
        }

        if (
          pushIntoQueue && // only the new task should be pushed
          payload.indexSet.size !== 0
        ) {
          // some data has not been fetched yet
          this.queueAlbumsUpdate.push(payload)
        }
      },
      selfUpdate() {
        this.queueTagsUpdate.forEach((payload) => {
          this.optimisticUpdateTags(payload, false)
        })
        this.queueAlbumsUpdate.forEach((payload) => {
          this.optimisticUpdateAlbums(payload, false)
        })
        this.queueStatusUpdate.forEach((payload) => {
          this.optimisticUpdateStatus(payload, false)
        })
      },
      optimisticUpdateStatus(payload: EditStatusPayload, pushIntoQueue: boolean = true) {
        const dataStore = useDataStore(isolationId)
        const pendingIndexes: number[] = []

        for (const index of payload.indexArray) {
          const result = dataStore.setStatus(index, {
            isFavorite: payload.isFavorite,
            isArchived: payload.isArchived,
            isTrashed: payload.isTrashed
          })
          if (!result) {
            pendingIndexes.push(index)
          }
        }

        if (pushIntoQueue && pendingIndexes.length > 0) {
          this.queueStatusUpdate.push({
            ...payload,
            indexArray: pendingIndexes
          })
        }
      }
    }
  })()
