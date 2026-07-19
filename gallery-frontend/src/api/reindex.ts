import axios from 'axios'
import type { ReindexRequestContext } from '@/store/modalStore'

export type ReindexOperation =
  | 'exif'
  | 'dimensions'
  | 'fileSize'
  | 'thumbnail'
  | 'visualHashes'
  | 'videoCompression'
  | 'clearTags'

export const ALL_REINDEX_OPERATIONS: readonly ReindexOperation[] = [
  'exif',
  'dimensions',
  'fileSize',
  'thumbnail',
  'visualHashes',
  'videoCompression',
  'clearTags'
]

export const SAFE_REINDEX_OPERATIONS: readonly ReindexOperation[] = [
  'exif',
  'dimensions',
  'fileSize',
  'thumbnail',
  'visualHashes'
]

export type ReindexJobState =
  | 'queued'
  | 'running'
  | 'completed'
  | 'completedWithErrors'
  | 'canceled'
  | 'failed'

export interface ReindexJobError {
  objectId: string
  stage: string
  message: string
}

export interface ReindexJobStatus {
  jobId: string
  state: ReindexJobState
  queuePosition: number | null
  operations: ReindexOperation[]
  total: number
  processed: number
  succeeded: number
  failed: number
  skipped: number
  createdAt: number
  startedAt: number | null
  finishedAt: number | null
  cancelRequested: boolean
  errors: ReindexJobError[]
}

export interface ReindexJobAccepted {
  jobId: string
  targetCount: number
}

export const isTerminalReindexState = (state: ReindexJobState): boolean =>
  ['completed', 'completedWithErrors', 'canceled', 'failed'].includes(state)

export const reindexProgress = (job: ReindexJobStatus): number =>
  job.total === 0 ? 100 : Math.min(100, Math.round((job.processed / job.total) * 100))

export const normalizeReindexOperations = (
  operations: readonly ReindexOperation[]
): ReindexOperation[] => {
  const selected = new Set(operations)
  return ALL_REINDEX_OPERATIONS.filter((operation) => selected.has(operation))
}

export const isDangerousReindexPlan = (operations: readonly ReindexOperation[]): boolean =>
  operations.includes('clearTags')

export const submitReindexJob = async (
  context: ReindexRequestContext,
  operations: readonly ReindexOperation[]
): Promise<ReindexJobAccepted> => {
  const normalized = normalizeReindexOperations(operations)
  if (normalized.length === 0) throw new Error('Select at least one reindex operation')
  const response = await axios.post<ReindexJobAccepted>('/put/reindex', {
    selection: context.selection,
    timestamp: context.timestamp,
    operations: normalized
  })
  return response.data
}

export const fetchReindexJobs = async (): Promise<ReindexJobStatus[]> => {
  const response = await axios.get<ReindexJobStatus[]>('/get/reindex/jobs')
  return response.data
}

export const cancelReindexJob = async (jobId: string): Promise<ReindexJobStatus> => {
  const response = await axios.post<ReindexJobStatus>(`/put/reindex/${jobId}/cancel`)
  return response.data
}
