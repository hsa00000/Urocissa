import { describe, expect, it, vi } from 'vitest'
import { dispatchImageRequestsInParallel } from './parallelImageRequests'

function deferred(): {
  promise: Promise<void>
  resolve: () => void
  reject: (error: Error) => void
} {
  let resolve!: () => void
  let reject!: (error: Error) => void
  const promise = new Promise<void>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })
  return { promise, resolve, reject }
}

describe('dispatchImageRequestsInParallel', () => {
  it('starts every request without waiting for a prior request to settle', async () => {
    const requests = [deferred(), deferred(), deferred(), deferred()]
    const started: number[] = []

    const settledPromise = dispatchImageRequestsInParallel(requests, async (request) => {
      started.push(requests.indexOf(request))
      await request.promise
    })

    expect(started).toEqual([0, 1, 2, 3])
    requests.forEach((request) => {
      request.resolve()
    })
    await expect(settledPromise).resolves.toEqual([
      { status: 'fulfilled', value: undefined },
      { status: 'fulfilled', value: undefined },
      { status: 'fulfilled', value: undefined },
      { status: 'fulfilled', value: undefined }
    ])
  })

  it('allows other requests to finish when one request fails', async () => {
    const completed = vi.fn()

    const results = await dispatchImageRequestsInParallel([0, 1, 2], (index) => {
      if (index === 1) {
        return Promise.reject(new Error('token failed'))
      }
      completed(index)
      return Promise.resolve()
    })

    expect(completed.mock.calls).toEqual([[0], [2]])
    expect(results.map((result) => result.status)).toEqual(['fulfilled', 'rejected', 'fulfilled'])
  })
})
