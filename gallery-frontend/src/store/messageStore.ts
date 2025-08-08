import { Message, MessageColor } from '@type/types'
import { defineStore } from 'pinia'

export const useMessageStore = defineStore('messageStore', {
  state: (): { queue: Message[] } => ({
    queue: []
  }),
  actions: {
    push(text: string, color: MessageColor) {
      this.queue.push({ text, color })
    },
    error(text: string) {
      this.push(text, 'error')
    },
    success(text: string) {
      this.push(text, 'success')
    },
    info(text: string) {
      this.push(text, 'info')
    }
  }
})
