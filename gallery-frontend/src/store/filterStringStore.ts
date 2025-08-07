import { defineStore } from 'pinia'

export const useFilterStringStore = defineStore('filterStringStore', {
  state: (): {
    filterString: string | null
  } => ({
    filterString: null
  }),
  actions: {}
})
