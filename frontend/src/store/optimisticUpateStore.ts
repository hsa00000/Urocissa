import { EnrichedUnifiedData, IsolationId } from '@type/types'
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

export const useOptimisticStore = (isolationId: IsolationId) =>
  defineStore('optimisticUpdateStore' + isolationId, {
    state: (): {
      backupData: Map<number, EnrichedUnifiedData> // dataIndex -> data
      queueTagsUpdate: EditTagsPayload[]
      queueAlbumsUpdate: EditAlbumsPayload[]
    } => ({
      backupData: new Map(),
      queueTagsUpdate: [],
      queueAlbumsUpdate: []
    }),
    actions: {
      clearAll() {
        this.backupData.clear()
        this.queueTagsUpdate = []
        this.queueAlbumsUpdate = []
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
      }
    }
  })()
