import axios, { type AxiosProgressEvent } from 'axios'
import { defineStore } from 'pinia'
import { markRaw } from 'vue'
import { errorDisplay } from '@/script/utils/errorDisplay'
import { useModalStore } from './modalStore'
import type { IsolationId } from '@type/types'

export const IMAGE_EXTENSIONS = [
  'jpg',
  'jpeg',
  'jfif',
  'jpe',
  'png',
  'tif',
  'tiff',
  'webp',
  'bmp'
] as const

export const VIDEO_EXTENSIONS = [
  'gif',
  'mp4',
  'webm',
  'mkv',
  'mov',
  'avi',
  'flv',
  'wmv',
  'mpeg'
] as const

export const SUPPORTED_MEDIA_EXTENSIONS = [...IMAGE_EXTENSIONS, ...VIDEO_EXTENSIONS] as const
export const UPLOAD_ACCEPT = SUPPORTED_MEDIA_EXTENSIONS.map((extension) => `.${extension}`).join(',')

const supportedExtensionSet = new Set<string>(SUPPORTED_MEDIA_EXTENSIONS)

export type UploadStatus =
  | 'pending'
  | 'uploading'
  | 'processing'
  | 'success'
  | 'error'
  | 'canceled'

export interface UploadShareContext {
  albumId: string
  shareId: string
  password?: string | null
}

export interface UploadPresetAlbum {
  id: string
  name?: string | null
}

export interface UploadTarget {
  albums?: UploadPresetAlbum[]
  tags?: string[]
  share?: UploadShareContext
}

export interface UploadPresentationOptions {
  showSummaryPanel?: boolean
}

export interface UploadQueueItem {
  id: string
  runId: string
  file: File
  target: UploadTarget
  status: UploadStatus
  loaded: number
  total: number
  errorReason?: string
  startedAt?: number
  endedAt?: number
}

interface UploadStoreState {
  items: UploadQueueItem[]
  pendingIds: string[]
  activeItemId: string | null
  activeAbortController: AbortController | null
  runnerActive: boolean
  currentRunId: string | null
  currentRunStartedAt: number | null
  presignAlbumIds: string[]
  presignTags: string[]
}

const ACTIVE_STATUSES: ReadonlySet<UploadStatus> = new Set([
  'pending',
  'uploading',
  'processing'
])
const ISSUE_STATUSES: ReadonlySet<UploadStatus> = new Set(['error', 'canceled'])

function isActiveStatus(status: UploadStatus): boolean {
  return ACTIVE_STATUSES.has(status)
}

function isTerminalStatus(status: UploadStatus): boolean {
  return !isActiveStatus(status)
}

function shouldShowSummaryPanel(options: UploadPresentationOptions | undefined): boolean {
  return options?.showSummaryPanel !== false
}

function getFileExtension(fileName: string): string {
  const separatorIndex = fileName.lastIndexOf('.')
  if (separatorIndex < 0 || separatorIndex === fileName.length - 1) return ''
  return fileName.slice(separatorIndex + 1).toLowerCase()
}

export function isSupportedUploadFile(file: Pick<File, 'name'>): boolean {
  return supportedExtensionSet.has(getFileExtension(file.name))
}

function cloneTarget(target: UploadTarget | undefined): UploadTarget {
  if (target === undefined) return markRaw<UploadTarget>({})

  const cloned: UploadTarget = {}
  if (target.albums !== undefined) {
    const albumIds = new Set<string>()
    const albums: UploadPresetAlbum[] = []
    for (const album of target.albums) {
      if (album.id === '' || albumIds.has(album.id)) continue
      albumIds.add(album.id)
      albums.push(markRaw({ id: album.id, name: album.name }))
    }
    if (albums.length > 0) cloned.albums = markRaw(albums)
  }
  if (target.tags !== undefined) {
    const tags = [...new Set(target.tags.map((tag) => tag.trim()).filter((tag) => tag !== ''))]
    if (tags.length > 0) cloned.tags = markRaw(tags)
  }
  if (target.share !== undefined) {
    cloned.share = markRaw({
      albumId: target.share.albumId,
      shareId: target.share.shareId,
      password: target.share.password
    })
  }
  return markRaw(cloned)
}

function buildUploadUrl(target: UploadTarget): string {
  if (target.share !== undefined) {
    return `/upload?presigned_album_id_opt=${encodeURIComponent(target.share.albumId)}`
  }

  const albumIds = target.albums?.map((album) => album.id) ?? []
  const tags = target.tags ?? []
  if (albumIds.length === 0 && tags.length === 0) return '/upload'

  const query = new URLSearchParams()
  if (albumIds.length > 0) {
    query.set('presigned_album_ids_opt', JSON.stringify(albumIds))
  }
  if (tags.length > 0) {
    query.set('presigned_tags_opt', JSON.stringify(tags))
  }
  return `/upload?${query.toString()}`
}

function createId(prefix: string): string {
  return `${prefix}-${crypto.randomUUID()}`
}

export const useUploadStore = (isolationId: IsolationId) =>
  defineStore('uploadStore' + isolationId, {
    state: (): UploadStoreState => ({
      items: [],
      pendingIds: [],
      activeItemId: null,
      activeAbortController: null,
      runnerActive: false,
      currentRunId: null,
      currentRunStartedAt: null,
      presignAlbumIds: [],
      presignTags: []
    }),

    getters: {
      uploadingItems(state): UploadQueueItem[] {
        return state.items.filter((item) => isActiveStatus(item.status))
      },

      successItems(state): UploadQueueItem[] {
        return state.items.filter((item) => item.status === 'success')
      },

      issueItems(state): UploadQueueItem[] {
        return state.items.filter((item) => ISSUE_STATUSES.has(item.status))
      },

      currentItem(state): UploadQueueItem | undefined {
        if (state.activeItemId === null) return undefined
        return state.items.find((item) => item.id === state.activeItemId)
      },

      hasActiveWork(state): boolean {
        return state.items.some((item) => isActiveStatus(item.status))
      },

      currentRunItems(state): UploadQueueItem[] {
        if (state.currentRunId === null) return []
        return state.items.filter((item) => item.runId === state.currentRunId)
      },

      currentRunTotalCount(): number {
        return this.currentRunItems.length
      },

      currentRunCompletedCount(): number {
        return this.currentRunItems.filter((item) => isTerminalStatus(item.status)).length
      },

      currentRunSuccessCount(): number {
        return this.currentRunItems.filter((item) => item.status === 'success').length
      },

      currentRunErrorCount(): number {
        return this.currentRunItems.filter((item) => ISSUE_STATUSES.has(item.status)).length
      },

      currentRunTotalBytes(): number {
        return this.currentRunItems.reduce((total, item) => total + item.total, 0)
      },

      currentRunTransferredBytes(): number {
        return this.currentRunItems.reduce(
          (total, item) => total + Math.min(item.loaded, item.total),
          0
        )
      },

      currentRunProgressPercent(): number {
        const total = this.currentRunTotalBytes
        if (total <= 0) return this.currentRunTotalCount > 0 ? 100 : 0

        const accountedBytes = this.currentRunItems.reduce((sum, item) => {
          if (isTerminalStatus(item.status)) return sum + item.total
          return sum + Math.min(item.loaded, item.total)
        }, 0)
        return Math.min(100, Math.floor((accountedBytes / total) * 100))
      },

      currentRunIsComplete(): boolean {
        return (
          this.currentRunTotalCount > 0 &&
          this.currentRunItems.every((item) => isTerminalStatus(item.status))
        )
      },

      elapsedTime(state): number {
        if (state.currentRunStartedAt === null) return 0
        return Math.max(0, (Date.now() - state.currentRunStartedAt) / 1000)
      },

      uploadSpeed(): number {
        return this.elapsedTime > 0 ? this.currentRunTransferredBytes / this.elapsedTime : 0
      },

      remainingTime(): number {
        if (!this.hasActiveWork || this.uploadSpeed <= 0) return 0
        const remainingBytes = this.currentRunItems.reduce((total, item) => {
          if (!isActiveStatus(item.status)) return total
          return total + Math.max(0, item.total - item.loaded)
        }, 0)
        return remainingBytes / this.uploadSpeed
      },

      // Compatibility alias used by the compact upload panel.
      percentComplete(): number {
        return this.currentRunProgressPercent
      }
    },

    actions: {
      ensureCurrentRun(): string {
        const existingRunId = this.currentRunId
        const runStillActive =
          existingRunId !== null &&
          this.items.some(
            (item) => item.runId === existingRunId && isActiveStatus(item.status)
          )

        if (existingRunId !== null && runStillActive) return existingRunId

        const runId = createId('upload-run')
        this.currentRunId = runId
        this.currentRunStartedAt = Date.now()
        return runId
      },

      openFilePicker(
        target?: UploadTarget,
        presentation?: UploadPresentationOptions
      ): void {
        const input = document.createElement('input')
        input.type = 'file'
        input.multiple = true
        input.accept = UPLOAD_ACCEPT
        input.style.display = 'none'

        const cleanup = (): void => {
          input.remove()
        }

        input.addEventListener(
          'change',
          () => {
            const files = Array.from(input.files ?? [])
            cleanup()
            this.enqueueFiles(files, target, presentation)
          },
          { once: true }
        )
        input.addEventListener('cancel', cleanup, { once: true })
        document.body.appendChild(input)
        input.click()
      },

      enqueueFiles(
        files: readonly File[],
        target?: UploadTarget,
        presentation?: UploadPresentationOptions
      ): void {
        if (files.length === 0) return

        const runId = this.ensureCurrentRun()
        const now = Date.now()

        for (const file of files) {
          const supported = isSupportedUploadFile(file)
          const item: UploadQueueItem = {
            id: createId('upload-item'),
            runId,
            file: markRaw(file),
            target: cloneTarget(target),
            status: supported ? 'pending' : 'error',
            loaded: 0,
            total: file.size,
            errorReason: supported ? undefined : 'Unsupported file format',
            endedAt: supported ? undefined : now
          }
          this.items.push(item)
          if (supported) this.pendingIds.push(item.id)
        }

        if (shouldShowSummaryPanel(presentation)) {
          useModalStore('mainId').showUploadModal = true
        }
        void this.processQueue()
      },

      // Kept for existing callers while they migrate to the typed target API.
      triggerFileInput(albumId: string | undefined): void {
        this.openFilePicker(
          albumId === undefined ? undefined : { albums: [{ id: albumId }] }
        )
      },

      // Kept for the existing drop-zone call site.
      fileUpload(files: File[], albumId: string | undefined): Promise<void> {
        this.enqueueFiles(
          files,
          albumId === undefined ? undefined : { albums: [{ id: albumId }] }
        )
        return Promise.resolve()
      },

      async processQueue(): Promise<void> {
        if (this.runnerActive) return
        this.runnerActive = true

        try {
          let itemId = this.pendingIds.shift()
          while (itemId !== undefined) {
            const item = this.items.find((candidate) => candidate.id === itemId)
            if (item?.status === 'pending') await this.uploadItem(itemId)
            itemId = this.pendingIds.shift()
          }
        } finally {
          this.runnerActive = false
          this.activeItemId = null
          this.activeAbortController = null

          // An enqueue can happen as the previous runner is settling.
          if (this.pendingIds.length > 0) void this.processQueue()
        }
      },

      async uploadItem(itemId: string): Promise<void> {
        const item = this.items.find((candidate) => candidate.id === itemId)
        if (item?.status !== 'pending') return

        const abortController = markRaw(new AbortController())
        this.activeItemId = item.id
        this.activeAbortController = abortController
        item.status = 'uploading'
        item.loaded = 0
        item.total = item.file.size
        item.errorReason = undefined
        item.startedAt = Date.now()
        item.endedAt = undefined

        const formData = new FormData()
        formData.append('file', item.file)
        formData.append('lastModified', String(item.file.lastModified))

        const uploadUrl = buildUploadUrl(item.target)

        const headers: Record<string, string> = { 'Content-Type': 'multipart/form-data' }
        if (item.target.share !== undefined) {
          headers['x-album-id'] = item.target.share.albumId
          headers['x-share-id'] = item.target.share.shareId
          const password = item.target.share.password
          if (password !== undefined && password !== null && password !== '') {
            headers['x-share-password'] = password
          }
        }

        try {
          await axios.post(uploadUrl, formData, {
            headers,
            signal: abortController.signal,
            onUploadProgress: (event: AxiosProgressEvent) => {
              if (item.status !== 'uploading' && item.status !== 'processing') return
              const total = event.total ?? item.file.size
              item.total = total
              item.loaded = Math.min(event.loaded, total)
              if (total > 0 && item.loaded >= total) item.status = 'processing'
            }
          })

          if (!abortController.signal.aborted) {
            item.loaded = item.total
            item.status = 'success'
          }
        } catch (error: unknown) {
          if (abortController.signal.aborted || axios.isCancel(error)) {
            item.status = 'canceled'
          } else {
            item.status = 'error'
            item.errorReason = errorDisplay(error)
          }
        } finally {
          item.endedAt = Date.now()
          if (this.activeItemId === item.id) {
            this.activeItemId = null
            this.activeAbortController = null
          }
        }
      },

      cancelItem(itemId: string): void {
        const item = this.items.find((candidate) => candidate.id === itemId)
        if (item === undefined || !isActiveStatus(item.status)) return

        this.pendingIds = this.pendingIds.filter((pendingId) => pendingId !== itemId)
        item.status = 'canceled'
        item.endedAt = Date.now()

        if (this.activeItemId === itemId) this.activeAbortController?.abort()
      },

      cancelAll(): void {
        for (const item of [...this.items]) {
          if (isActiveStatus(item.status)) this.cancelItem(item.id)
        }
      },

      // Compatibility alias used by the previous compact panel.
      cancelUpload(): void {
        this.cancelAll()
      },

      retryItem(itemId: string, presentation?: UploadPresentationOptions): void {
        const item = this.items.find((candidate) => candidate.id === itemId)
        if (
          item === undefined ||
          !ISSUE_STATUSES.has(item.status) ||
          this.activeItemId === itemId
        ) {
          return
        }

        item.runId = this.ensureCurrentRun()
        item.status = 'pending'
        item.loaded = 0
        item.total = item.file.size
        item.errorReason = undefined
        item.startedAt = undefined
        item.endedAt = undefined
        this.pendingIds.push(item.id)

        if (shouldShowSummaryPanel(presentation)) {
          useModalStore('mainId').showUploadModal = true
        }
        void this.processQueue()
      },

      retryAll(presentation?: UploadPresentationOptions): void {
        const retryIds = this.issueItems.map((item) => item.id)
        for (const itemId of retryIds) this.retryItem(itemId, presentation)
      },

      clearItem(itemId: string): void {
        const item = this.items.find((candidate) => candidate.id === itemId)
        if (item === undefined || !isTerminalStatus(item.status)) return
        this.items = this.items.filter((candidate) => candidate.id !== itemId)
        this.pendingIds = this.pendingIds.filter((pendingId) => pendingId !== itemId)
      },

      clearCompleted(): void {
        this.items = this.items.filter((item) => item.status !== 'success')
      },

      clearIssues(): void {
        this.items = this.items.filter((item) => !ISSUE_STATUSES.has(item.status))
      },

      // Compatibility alias used by the Litecissa-inspired success tab.
      clearAll(): void {
        this.clearCompleted()
      }
    }
  })()
