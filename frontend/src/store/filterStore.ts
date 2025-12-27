import { IsolationId } from '@type/types'
import { generateJsonString } from '@/script/lexer/generateJson'
import { defineStore } from 'pinia'
import { LocationQueryValue } from 'vue-router'

export const useFilterStore = (isolationId: IsolationId) =>
  defineStore('filterStore' + isolationId, {
    state: (): {
      // Records the gallery search filter
      searchString: LocationQueryValue | LocationQueryValue[] | undefined
    } => ({
      searchString: null
    }),
    actions: {
      // Generates the filter JSON string using basicString and searchString
      // This JSON info is used to send to the backend
      generateFilterJsonString(basicString: string | null): string | null {
        const hasBasicString = typeof basicString === 'string'
        const searchStringStr = typeof this.searchString === 'string' ? this.searchString : null
        const hasSearchString = searchStringStr !== null
        const stripQuotes = (s: string) => s.replace(/"/g, '')

        if (!hasBasicString && !hasSearchString) return null

        if (hasBasicString && !hasSearchString) {
          try {
            return generateJsonString(basicString)
          } catch (err) {
            console.error(err)
            return null
          }
        }

        if (!hasBasicString && hasSearchString) {
          try {
            return generateJsonString(searchStringStr)
          } catch {
            const s = stripQuotes(searchStringStr)
            return generateJsonString(`any: "${s}"`)
          }
        }

        if (hasBasicString && hasSearchString) {
          try {
            return generateJsonString(`and(${basicString},${searchStringStr})`)
          } catch {
            const s = stripQuotes(searchStringStr)
            return generateJsonString(`and(${basicString}, any: "${s}")`)
          }
        }

        return null
      }
    }
  })()
