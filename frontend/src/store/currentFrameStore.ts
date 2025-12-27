import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useCurrentFrameStore = (isolationId: IsolationId) =>
  defineStore('currentFrameStore' + isolationId, {
    state: (): {
      video: HTMLVideoElement | null // unit: second
    } => ({
      video: null
    }),
    actions: {
      async getCapture() {
        if (this.video) {
          const canvas = document.createElement('canvas')
          canvas.width = this.video.videoWidth
          canvas.height = this.video.videoHeight
          const context = canvas.getContext('2d')
          if (context) {
            // Draw the current video frame onto the canvas
            context.drawImage(this.video, 0, 0, canvas.width, canvas.height)

            const blob = await new Promise<Blob | null>((resolve) => {
              canvas.toBlob((blob) => {
                resolve(blob)
              }, 'image/jpeg')
            })

            if (blob) {
              return blob
            }
          }
        }
      }
    }
  })()
