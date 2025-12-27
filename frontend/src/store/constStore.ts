import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'
import {
  storeSubRowHeightScale,
  getSubRowHeightScale,
  storeShowInfo,
  getShowInfo,
  storeConcurrencyNumber,
  getConcurrencyNumber,
  storeLimitRation,
  getLimitRation,
  storeTheme,
  getTheme,
  storeShowFilenameChip,
  getShowFilenameChip
} from '@/db/settingsDb'

export const useConstStore = (isolationId: IsolationId) =>
  defineStore('constStore' + isolationId, {
    state: (): {
      subRowHeightScale: number
      showInfo: boolean
      concurrencyNumber: number
      limitRatio: boolean
      theme: 'dark' | 'light'
      showFilenameChip: boolean
    } => ({
      subRowHeightScale: 250,
      showInfo: false,
      concurrencyNumber: Math.max(Math.floor(navigator.hardwareConcurrency / 2), 1),
      // default: false
      limitRatio: false,
      // default theme: dark
      theme: 'dark',
      showFilenameChip: false
    }),
    actions: {
      async toggleTheme(vuetifyTheme?: { global: { name: { value: string } } }): Promise<void> {
        const newTheme = this.theme === 'light' ? 'dark' : 'light'
        this.theme = newTheme
        await storeTheme(newTheme)

        // Update Vuetify theme if provided
        if (vuetifyTheme) {
          vuetifyTheme.global.name.value = newTheme
        }
      },

      async updateSubRowHeightScale(value: number): Promise<void> {
        this.subRowHeightScale = value
        await storeSubRowHeightScale(value)
      },

      async loadSubRowHeightScale(): Promise<void> {
        const storedValue = await getSubRowHeightScale()
        if (storedValue !== null) {
          this.subRowHeightScale = storedValue
        }
      },

      async updateShowInfo(value: boolean): Promise<void> {
        this.showInfo = value
        await storeShowInfo(value)
      },

      async loadShowInfo(): Promise<void> {
        const stored = await getShowInfo()
        if (stored !== null) {
          this.showInfo = stored
        }
      },

      // ===== 新增：concurrencyNumber =====
      async updateConcurrencyNumber(value: number): Promise<void> {
        // 保障為 >= 1 的整數
        const v = Math.max(1, Math.floor(value))
        this.concurrencyNumber = v
        await storeConcurrencyNumber(v)
      },

      async loadConcurrencyNumber(): Promise<void> {
        const stored = await getConcurrencyNumber()
        if (typeof stored === 'number' && Number.isFinite(stored) && stored > 0) {
          this.concurrencyNumber = Math.floor(stored)
        }
      },
      async updateLimitRation(value: boolean): Promise<void> {
        this.limitRatio = value
        await storeLimitRation(this.limitRatio)
      },

      async loadLimitRation(): Promise<void> {
        const stored = await getLimitRation()
        if (typeof stored === 'boolean') {
          this.limitRatio = stored
        }
      },
      async updateTheme(value: 'dark' | 'light'): Promise<void> {
        this.theme = value
        await storeTheme(value)
      },

      async loadTheme(): Promise<void> {
        const stored = await getTheme()
        if (stored === 'dark' || stored === 'light') {
          this.theme = stored
        }
      },

      async updateShowFilenameChip(value: boolean): Promise<void> {
        this.showFilenameChip = value
        await storeShowFilenameChip(value)
      },

      async loadShowFilenameChip(): Promise<void> {
        const stored = await getShowFilenameChip()
        if (typeof stored === 'boolean') {
          this.showFilenameChip = stored
        }
      }
    }
  })()
