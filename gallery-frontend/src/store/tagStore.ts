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
      },
      optimisticAddTags(newTags: string[]) {
        let hasChange = false
        newTags.forEach((newTag) => {
          // 檢查標籤是否已存在
          const exists = this.tags.find((t) => t.tag === newTag)
          if (exists) {
            // 如果存在，增加計數 (不精確)
            exists.number += 1
          } else {
            // 如果不存在，推入新標籤
            this.tags.push({ tag: newTag, number: 1 })
            hasChange = true
          }
        })

        // 如果有新增標籤，重新排序以符合 UI 預期
        if (hasChange) {
          this.tags.sort((a, b) => a.tag.localeCompare(b.tag))
        }
      },

      optimisticRemoveTags(removeTags: string[]) {
        removeTags.forEach((tagToRemove) => {
          const target = this.tags.find((t) => t.tag === tagToRemove)
          if (target && target.number > 0) {
            target.number -= 1
          }
        })
      }
    }
  })()
