import type { DataBase } from '@/script/common/commonType'
import { defineStore } from 'pinia'

export const useDataStore = defineStore('DataStore', {
  state: (): {
    data: Map<number, DataBase> // dataIndex -> data
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
    }
  }
})
