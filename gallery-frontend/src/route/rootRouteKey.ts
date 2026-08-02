interface RootRouteKeyInput {
  baseName: unknown
  concurrencyNumber: number
}

/**
 * The root route layer owns component and worker topology, not collection data.
 * Search/sort/locate changes are handled in-place by usePrefetch so nested
 * overlays are never destroyed for a data reload.
 */
export function createRootRouteKey({
  baseName,
  concurrencyNumber
}: RootRouteKeyInput): string {
  return JSON.stringify([String(baseName), concurrencyNumber])
}
