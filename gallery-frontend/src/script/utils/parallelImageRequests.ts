export function dispatchImageRequestsInParallel<T>(
  requests: readonly T[],
  dispatch: (request: T) => Promise<void>
): Promise<PromiseSettledResult<void>[]> {
  return Promise.allSettled(requests.map((request) => dispatch(request)))
}
