import { describe, expect, it } from 'vitest'
import {
  SAFE_REINDEX_OPERATIONS,
  isDangerousReindexPlan,
  isTerminalReindexState,
  normalizeReindexOperations,
  reindexProgress,
  submitReindexJob
} from './reindex'
import type { ReindexJobStatus } from './reindex'

const job = (processed: number, total: number): ReindexJobStatus => ({
  jobId: 'job',
  state: 'running',
  queuePosition: null,
  operations: ['thumbnail'],
  total,
  processed,
  succeeded: processed,
  failed: 0,
  skipped: 0,
  createdAt: 1,
  startedAt: 2,
  finishedAt: null,
  cancelRequested: false,
  errors: []
})

describe('reindex plan and status derivation', () => {
  it('uses the five safe defaults and canonicalizes arbitrary plans', () => {
    expect(SAFE_REINDEX_OPERATIONS).toEqual([
      'exif',
      'dimensions',
      'fileSize',
      'thumbnail',
      'visualHashes'
    ])
    expect(normalizeReindexOperations(['clearTags', 'exif', 'exif'])).toEqual([
      'exif',
      'clearTags'
    ])
    expect(isDangerousReindexPlan(SAFE_REINDEX_OPERATIONS)).toBe(false)
    expect(isDangerousReindexPlan(['clearTags'])).toBe(true)
  })

  it('derives terminal states and bounded progress', () => {
    expect(isTerminalReindexState('running')).toBe(false)
    expect(isTerminalReindexState('completedWithErrors')).toBe(true)
    expect(reindexProgress(job(3, 4))).toBe(75)
    expect(reindexProgress(job(0, 0))).toBe(100)
    expect(reindexProgress(job(9, 4))).toBe(100)
  })

  it('rejects an empty plan before making a request', async () => {
    await expect(
      submitReindexJob(
        {
          selection: { mode: 'explicit', indices: [1] },
          timestamp: 10,
          isolationId: 'mainId',
          targetCount: 1
        },
        []
      )
    ).rejects.toThrow('Select at least one reindex operation')
  })
})
