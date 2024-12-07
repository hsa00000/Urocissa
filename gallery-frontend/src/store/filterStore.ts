import { generateJsonString } from '@/script/lexer/generateJson'
import { defineStore } from 'pinia'
import { RouteLocationNormalizedLoaded } from 'vue-router'

export const useFilterStore = (isolationId: string) =>
  defineStore({
    id: 'filterStore' + isolationId,
    state: (): {
      // Records the type of gallery area: default, favorite, archived, etc.
      basicString: string | null
      // Records the gallery search filter
      filterString: string | null
    } => ({
      basicString: null,
      filterString: null
    }),
    actions: {
      // Generates the filter JSON string using basicString and filterString
      // This JSON info is used to send to the backend
      generateFilterJsonString(): string | null {
        if (this.basicString !== null && this.filterString === null) {
          return generateJsonString(this.basicString)
        } else if (this.basicString === null && this.filterString !== null) {
          return generateJsonString(this.filterString)
        } else if (this.basicString !== null && this.filterString !== null) {
          return generateJsonString(`and(${this.basicString},${this.filterString})`)
        } else {
          return null
        }
      },
      handleFilterString(route: RouteLocationNormalizedLoaded) {
        const searchString = route.query.search as string
        this.filterString = searchString ? searchString : null
      },
      handleBasicString(route: RouteLocationNormalizedLoaded, isolationId: string) {
        if (route.meta.isReadPage) {
          if (isolationId === 'mainId') {
            this.basicString = route.meta.basicString
          } else {
            const album_id = route.params.hash
            if (typeof album_id === 'string') {
              this.basicString = `and(album:"${album_id}", not(tag:"_trashed"))`
            }
          }
        } else {
          this.basicString = route.meta.basicString
        }
      }
    }
  })()
