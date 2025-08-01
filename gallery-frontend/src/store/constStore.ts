import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'
import { storeSubRowHeightScale, getSubRowHeightScale } from '@/db/settingsDb'

export const useConstStore = (isolationId: IsolationId) =>
  defineStore('constStore' + isolationId, {
    state: (): {
      disableImg: boolean
      subRowHeightScale: number
    } => ({
      disableImg: false,
      subRowHeightScale: 250
    }),
    actions: {
      async updateSubRowHeightScale(value: number): Promise<void> {
        this.subRowHeightScale = value
        await storeSubRowHeightScale(value)
      },

      async loadSubRowHeightScale(): Promise<void> {
        const storedValue = await getSubRowHeightScale()
        if (storedValue !== null) {
          this.subRowHeightScale = storedValue
        }
      }
    }
  })()
