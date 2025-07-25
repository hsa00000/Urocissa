import { IsolationId, TagInfo } from '@type/types'
import { tagInfoSchema } from '@type/schemas'
import axios from 'axios'
import { defineStore } from 'pinia'
import { z } from 'zod'
import { tryWithMessageStore } from '@/script/utils/try_catch'

export const useTagStore = (isolationId: IsolationId) =>
  defineStore('tagStore' + isolationId, {
    state: (): {
      tags: TagInfo[]
      fetched: boolean
    } => ({
      tags: [],
      fetched: false
    }),
    actions: {
      async fetchTags() {
        await tryWithMessageStore('mainId', async () => {
          const response = await axios.get('/get/get-tags')

          if (response.status !== 200) {
            throw new Error('Network response was not ok')
          }

          const tagsArraySchema = z.array(tagInfoSchema)
          const tags = tagsArraySchema.parse(response.data)

          this.tags = tags
          this.tags.sort((a, b) => a.tag.localeCompare(b.tag))
          this.fetched = true
        })
      },
      clearAll() {
        this.tags = []
        this.fetched = false
      },
      applyTags(tagsJson: { tag: string; number: number }[]) {
        this.tags = tagsJson
        this.tags.sort((a, b) => a.tag.localeCompare(b.tag))
        this.fetched = true
      }
    }
  })()
