// src/store/uploadStore.ts
import { defineStore } from 'pinia'
import axios, { type AxiosProgressEvent } from 'axios'
import { useMessageStore } from './messageStore'
import { useModalStore } from './modalStore'
import { errorDisplay } from '@/script/utils/errorDisplay'
import { IsolationId } from '@type/types'

export const useUploadStore = (isolationId: IsolationId) =>
  defineStore('uploadStore' + isolationId, {
    state: () => ({
      status: 'Canceled' as 'Uploading' | 'Processing' | 'Canceled' | 'Completed',
      total: undefined as number | undefined,
      loaded: undefined as number | undefined,
      startTime: undefined as number | undefined,
      abortController: null as AbortController | null
    }),

    // ===== Getters（屬性存取，非函式呼叫）=====
    getters: {
      percentComplete: (state): number =>
        state.total !== undefined && state.loaded !== undefined && state.total > 0
          ? Math.floor((state.loaded / state.total) * 100)
          : 0,

      elapsedTime: (state): number =>
        state.startTime !== undefined ? (Date.now() - state.startTime) / 1000 : 0,

      uploadSpeed(): number {
        const elapsed = this.elapsedTime
        return elapsed > 0 && this.loaded !== undefined ? this.loaded / elapsed : 0 // bytes/sec
      },

      remainingTime(): number {
        const speed = this.uploadSpeed
        if (speed > 0 && this.total !== undefined && this.loaded !== undefined) {
          return (this.total - this.loaded) / speed // seconds
        }
        return 0
      }
    },

    // ===== Actions =====
    actions: {
      // 顯式型別：albumId 必傳，但可為 undefined
      triggerFileInput(albumId: string | undefined): void {
        const fileInput = document.createElement('input')
        fileInput.type = 'file'
        fileInput.multiple = true
        fileInput.style.display = 'none'
        // fileInput.accept = 'image/*,video/*' // 需要時再開

        // 實際異步處理
        const handleChange = async (event: Event): Promise<void> => {
          const target = event.target as HTMLInputElement
          const files = target.files
          try {
            if (files && files.length > 0) {
              await this.fileUpload([...files], albumId) // albumId 由 closure 固定
            }
          } finally {
            document.body.removeChild(fileInput) // { once: true } 會自動移除監聽
          }
        }

        // 同步 wrapper，避免 no-misused-promises
        const changeHandler = (e: Event): void => {
          void handleChange(e)
        }

        fileInput.addEventListener('change', changeHandler, { once: true })
        document.body.appendChild(fileInput)
        fileInput.click()
      },

      // 顯式型別：albumId 必傳，但可為 undefined
      async fileUpload(files: File[], albumId: string | undefined): Promise<void> {
        const modalStore = useModalStore('mainId')
        const messageStore = useMessageStore('mainId')

        this.status = 'Uploading'
        modalStore.showUploadModal = true

        const formData = new FormData()
        for (const file of files) {
          formData.append('file', file)
          formData.append('lastModified', String(file.lastModified))
        }

        const uploadUrl =
          albumId !== undefined
            ? `/upload?presigned_album_id_opt=${encodeURIComponent(albumId)}`
            : `/upload`

        const abortController = new AbortController()
        this.abortController = abortController
        this.total = this.loaded = 0
        this.startTime = Date.now()

        try {
          await axios.post(uploadUrl, formData, {
            headers: { 'Content-Type': 'multipart/form-data' },
            signal: abortController.signal,
            onUploadProgress: (e: AxiosProgressEvent) => {
              if (e.total !== undefined) {
                this.total = e.total
                // e.loaded 可能為 undefined（類型上），保守處理
                if (typeof e.loaded === 'number') {
                  this.loaded = e.loaded
                }
                if (this.loaded !== undefined && this.total === this.loaded) {
                  this.status = 'Processing'
                }
              }
            }
          })

          this.status = 'Completed'
          messageStore.success('Files uploaded successfully')
        } catch (err) {
          this.status = 'Canceled'
          messageStore.error(errorDisplay(err))
        }
      },

      cancelUpload(): void {
        if (this.abortController) {
          this.abortController.abort()
          this.status = 'Canceled'
        }
      }
    }
  })()
