import { IsolationId } from '@type/types'
import { generateJsonString } from '@/script/lexer/generateJson'
import { defineStore } from 'pinia'
import { LocationQueryValue } from 'vue-router'

export const useSearchStringStore = (isolationId: IsolationId) =>
  defineStore('searchStringStore' + isolationId, {
    state: (): {
      // Records the gallery search filter
      searchString: LocationQueryValue | LocationQueryValue[] | undefined
    } => ({
      searchString: null
    }),
    actions: {
      // Generates the filter JSON string using filterString and searchString
      // This JSON info is used to send to the backend
      generateFilterJsonString(filterString: string | null): string | null {
        const hasBasicString = typeof filterString === 'string'
        const searchStringStr = typeof this.searchString === 'string' ? this.searchString : null
        const hasSearchString = searchStringStr !== null

        // No valid input
        if (!hasBasicString && !hasSearchString) {
          return null
        }

        // Only filterString
        if (hasBasicString && !hasSearchString) {
          return generateJsonString(filterString) || null
        }

        // Only searchString
        if (!hasBasicString && hasSearchString) {
          return (
            generateJsonString(searchStringStr) ||
            generateJsonString(`any: "${searchStringStr}"`) ||
            null
          )
        }

        // Both strings
        return (
          generateJsonString(`and(${filterString},${searchStringStr})`) ||
          generateJsonString(`and(${filterString}, any: "${searchStringStr}")`) ||
          null
        )
      }
    }
  })()
