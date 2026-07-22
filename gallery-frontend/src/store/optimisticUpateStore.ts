import { EnrichedUnifiedData, IsolationId } from '@type/types'
import { defineStore } from 'pinia'
import { useDataStore } from './dataStore'
import { useSearchFacetStore } from './searchFacetStore'
import type { SelectionDescriptor } from '@/type/selection'
import { createSelectionMatcher } from '@/type/selection'

export interface EditTagsPayload {
  selection: SelectionDescriptor
  addTagsArray: string[]
  removeTagsArray: string[]
  timestamp: number
}

export interface EditAlbumsPayload {
  selection: SelectionDescriptor
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
        const isSelected = createSelectionMatcher(payload.selection)
        for (const index of dataStore.data.keys()) {
          if (isSelected(index)) {
            dataStore.addTags(index, payload.addTagsArray)
            dataStore.removeTags(index, payload.removeTagsArray)
          }
        }

        // Optimistically add newly created tags so they appear
        // immediately in combobox dropdowns without waiting for a server round-trip.
        const searchFacetStore = useSearchFacetStore()
        for (const tag of payload.addTagsArray) {
          searchFacetStore.addOptimisticTag(tag)
        }

        void pushIntoQueue
      },
      optimisticUpdateAlbums(payload: EditAlbumsPayload, pushIntoQueue: boolean) {
        const dataStore = useDataStore(isolationId)
        const isSelected = createSelectionMatcher(payload.selection)
        for (const index of dataStore.data.keys()) {
          if (isSelected(index)) {
            dataStore.addAlbums(index, payload.addAlbumsArray)
            dataStore.removeAlbums(index, payload.removeAlbumsArray)
          }
        }

        void pushIntoQueue
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
