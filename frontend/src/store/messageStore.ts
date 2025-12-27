import { IsolationId, Message, MessageColor } from '@type/types'
import { defineStore } from 'pinia'

export const useMessageStore = (isolationId: IsolationId) =>
  defineStore('messageStore' + isolationId, {
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
  })()
