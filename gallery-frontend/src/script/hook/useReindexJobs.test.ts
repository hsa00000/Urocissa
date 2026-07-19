import { afterEach, describe, expect, it, vi } from 'vitest'
import { effectScope } from 'vue'
import type { ReindexJobStatus } from '@/api/reindex'
import type { ReindexRequestContext } from '@/store/modalStore'

const apiMocks = vi.hoisted(() => ({
  submitReindexJob: vi.fn(),
  fetchReindexJobs: vi.fn(),
  cancelReindexJob: vi.fn()
}))

vi.mock('@/api/reindex', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/reindex')>()),
  ...apiMocks
}))

import { useReindexJobs } from './useReindexJobs'

const context: ReindexRequestContext = {
  selection: { mode: 'explicit', indices: [1] },
  timestamp: 10,
  isolationId: 'mainId',
  targetCount: 1
}

const status = (state: ReindexJobStatus['state']): ReindexJobStatus => ({
  jobId: 'job-1',
  state,
  queuePosition: state === 'queued' ? 1 : null,
  operations: ['exif'],
  total: 1,
  processed: state === 'completed' ? 1 : 0,
  succeeded: state === 'completed' ? 1 : 0,
  failed: 0,
  skipped: 0,
  createdAt: 1,
  startedAt: state === 'queued' ? null : 2,
  finishedAt: state === 'completed' ? 3 : null,
  cancelRequested: state === 'canceled',
  errors: []
})

describe('useReindexJobs', () => {
  afterEach(() => {
    vi.clearAllMocks()
    vi.useRealTimers()
  })

  it('polls submitted work and emits one terminal refresh', async () => {
    vi.useFakeTimers()
    apiMocks.submitReindexJob.mockResolvedValue({ jobId: 'job-1', targetCount: 1 })
    apiMocks.fetchReindexJobs
      .mockResolvedValueOnce([status('queued')])
      .mockResolvedValueOnce([status('completed')])
    const terminal = vi.fn()
    const scope = effectScope()
    let jobs: ReturnType<typeof useReindexJobs> | undefined
    scope.run(() => {
      jobs = useReindexJobs({ onTerminalSuccess: terminal })
    })
    if (jobs === undefined) throw new Error('composable fixture was not initialized')

    await jobs.submit(context, ['exif'])
    expect(jobs.jobs.value[0]?.state).toBe('queued')
    expect(vi.getTimerCount()).toBe(1)
    await vi.advanceTimersByTimeAsync(1500)
    expect(jobs.jobs.value[0]?.state).toBe('completed')
    expect(terminal).toHaveBeenCalledTimes(1)
    expect(vi.getTimerCount()).toBe(0)
    scope.stop()
  })

  it('cancels a running job and refreshes its terminal status', async () => {
    apiMocks.fetchReindexJobs
      .mockResolvedValueOnce([status('running')])
      .mockResolvedValueOnce([status('canceled')])
    apiMocks.cancelReindexJob.mockResolvedValue(status('canceled'))
    const scope = effectScope()
    let jobs: ReturnType<typeof useReindexJobs> | undefined
    scope.run(() => {
      jobs = useReindexJobs()
    })
    if (jobs === undefined) throw new Error('composable fixture was not initialized')

    await jobs.refreshJobs()
    await jobs.cancel('job-1')
    expect(apiMocks.cancelReindexJob).toHaveBeenCalledWith('job-1')
    expect(jobs.jobs.value[0]?.state).toBe('canceled')
    expect(jobs.cancelingJobIds.value.size).toBe(0)
    scope.stop()
  })
})
