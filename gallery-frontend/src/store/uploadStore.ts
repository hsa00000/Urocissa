import { defineStore } from 'pinia'
import { useMessageStore } from './messageStore'
import axios from 'axios'
import { IsolationId } from '@type/types'
import { useModalStore } from './modalStore'

export const useUploadStore = (isolationId: IsolationId) =>
  defineStore('uploadStore' + isolationId, {
    state: (): {
      status: 'Uploading' | 'Processing' | 'Canceled' | 'Completed'
      total: number | undefined
      loaded: number | undefined
      startTime: number | undefined
      uploadButton: HTMLInputElement | null
      abortController: AbortController | null
    } => ({
      status: 'Canceled',
      total: undefined,
      loaded: undefined,
      startTime: undefined,
      uploadButton: null,
      abortController: null
    }),
    actions: {
      createUploadButton() {
        // Create the input element dynamically
        const fileInput = document.createElement('input')
        fileInput.type = 'file'
        fileInput.id = 'upload-input'
        fileInput.style.display = 'none'
        fileInput.multiple = true

        fileInput.addEventListener('change', (event) => {
          this.handleFileUpload(event)
            .then(() => ({}))
            .catch((error: unknown) => {
              console.error('Error:', error)
            })
        })

        // Append the input to the body
        document.body.appendChild(fileInput)

        // Set the uploadButton in the store
        this.uploadButton = fileInput
      },
      percentComplete() {
        if (this.total !== undefined && this.loaded !== undefined) {
          return Math.floor((this.loaded / this.total) * 100)
        }
        return 0
      },
      elapsedTime() {
        if (this.startTime !== undefined) {
          return (Date.now() - this.startTime) / 1000 // time in seconds
        }
        return 0
      },
      uploadSpeed() {
        const elapsed = this.elapsedTime()
        if (elapsed > 0 && this.loaded !== undefined) {
          return this.loaded / elapsed // speed in MB/s
        }
        return 0
      },
      remainingTime() {
        const speed = this.uploadSpeed()
        if (speed > 0 && this.total !== undefined && this.loaded !== undefined) {
          return (this.total - this.loaded) / speed // time in seconds
        }
        return 0
      },
      triggerFileInput(): void {
        if (!this.uploadButton) {
          this.createUploadButton()
        }

        if (this.uploadButton) {
          this.uploadButton.click()
        }
      },
      async fileUpload(files: File[]): Promise<void> {
        const modalStore = useModalStore('mainId')
        this.status = 'Uploading'
        modalStore.showUploadModal = true
        const messageStore = useMessageStore('mainId')
        const formData = new FormData()
        let totalSize = 0

        Array.from(files).forEach((file, i) => {
          formData.append(`lastModified${i}`, `${file.lastModified}`)
          formData.append(`file${i}`, file)
          totalSize += file.size
        })

        console.log(`Total upload size: ${totalSize} bytes`)
        const abortController = new AbortController()
        this.abortController = abortController
        try {
          const startTime = Date.now()

          this.total = 0
          this.loaded = 0
          this.startTime = startTime

          await axios.post('/upload', formData, {
            headers: {
              'Content-Type': 'multipart/form-data'
            },
            signal: abortController.signal,
            onUploadProgress: (progressEvent) => {
              if (progressEvent.total !== undefined) {
                this.total = progressEvent.total
                this.loaded = progressEvent.loaded
                this.startTime = startTime

                if (this.total === this.loaded) {
                  this.status = 'Processing'
                }

                console.log(`Upload is ${this.percentComplete()}% complete`)
                console.log(`Remaining time: ${this.remainingTime()} seconds`)
              }
            }
          })
          this.status = 'Completed'

          messageStore.success('Files uploaded successfully')
        } catch (error) {
          console.error('There was an error uploading the files: ', error)

          if (error instanceof Error) {
            messageStore.error(`There was an error uploading the files: ${error.message}`)
          } else {
            messageStore.error(`There was an error uploading the files: ${String(error)}`)
          }
        }
      },
      async handleFileUpload(event: Event): Promise<void> {
        const target = event.target as HTMLInputElement
        const files = target.files
        if (!files || files.length === 0) return
        await this.fileUpload([...files])
      },
      cancelUpload() {
        if (this.abortController) {
          this.abortController.abort()
          this.status = 'Canceled'
          console.log('Upload canceled by the user.')
        }
      }
    }
  })()
