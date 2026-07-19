import { computed, onScopeDispose, shallowRef } from 'vue'
import type { IsolationId } from '@/type/types'
import type { ReindexRequestContext } from '@/store/modalStore'
import {
  cancelReindexJob,
  fetchReindexJobs,
  isTerminalReindexState,
  submitReindexJob
} from '@/api/reindex'
import type {
  ReindexJobAccepted,
  ReindexJobStatus,
  ReindexOperation
} from '@/api/reindex'

const POLL_INTERVAL_MS = 1500

interface UseReindexJobsOptions {
  onTerminalSuccess?: (job: ReindexJobStatus, isolationId: IsolationId) => void | Promise<void>
}

export const useReindexJobs = (options: UseReindexJobsOptions = {}) => {
  const jobs = shallowRef<ReindexJobStatus[]>([])
  const loading = shallowRef(false)
  const submitting = shallowRef(false)
  const lastError = shallowRef<unknown>(null)
  const cancelingJobIds = shallowRef<ReadonlySet<string>>(new Set())
  const observedNonTerminal = new Set<string>()
  const jobIsolationIds = new Map<string, IsolationId>()
  let timer: ReturnType<typeof setInterval> | null = null
  let refreshInFlight = false

  const activeJobs = computed(() =>
    jobs.value.filter((job) => !isTerminalReindexState(job.state))
  )

  const stopPolling = () => {
    if (timer !== null) {
      clearInterval(timer)
      timer = null
    }
  }

  const pollSafely = () => {
    void refreshJobs().catch(() => undefined)
  }

  const startPolling = () => {
    timer ??= setInterval(pollSafely, POLL_INTERVAL_MS)
  }

  const refreshJobs = async (): Promise<ReindexJobStatus[]> => {
    if (refreshInFlight) return jobs.value
    refreshInFlight = true
    loading.value = true
    try {
      const nextJobs = await fetchReindexJobs()
      const terminalTransitions: ReindexJobStatus[] = []
      for (const job of nextJobs) {
        if (!isTerminalReindexState(job.state)) {
          observedNonTerminal.add(job.jobId)
        } else if (observedNonTerminal.delete(job.jobId) && job.succeeded > 0) {
          terminalTransitions.push(job)
        }
      }
      jobs.value = nextJobs
      lastError.value = null
      for (const job of terminalTransitions) {
        await options.onTerminalSuccess?.(
          job,
          jobIsolationIds.get(job.jobId) ?? 'mainId'
        )
      }
      if (nextJobs.some((job) => !isTerminalReindexState(job.state))) startPolling()
      else stopPolling()
      return nextJobs
    } catch (error: unknown) {
      lastError.value = error
      throw error
    } finally {
      loading.value = false
      refreshInFlight = false
    }
  }

  const submit = async (
    context: ReindexRequestContext,
    operations: readonly ReindexOperation[]
  ): Promise<ReindexJobAccepted> => {
    submitting.value = true
    try {
      const accepted = await submitReindexJob(context, operations)
      jobIsolationIds.set(accepted.jobId, context.isolationId)
      observedNonTerminal.add(accepted.jobId)
      startPolling()
      await refreshJobs()
      return accepted
    } finally {
      submitting.value = false
    }
  }

  const cancel = async (jobId: string): Promise<void> => {
    cancelingJobIds.value = new Set([...cancelingJobIds.value, jobId])
    try {
      await cancelReindexJob(jobId)
      await refreshJobs()
    } finally {
      const next = new Set(cancelingJobIds.value)
      next.delete(jobId)
      cancelingJobIds.value = next
    }
  }

  onScopeDispose(stopPolling)

  return {
    jobs,
    activeJobs,
    loading,
    submitting,
    lastError,
    cancelingJobIds,
    refreshJobs,
    startPolling,
    stopPolling,
    submit,
    cancel
  }
}
