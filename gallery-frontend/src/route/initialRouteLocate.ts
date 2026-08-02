let mainLocateOverride: string | null = null

export function saveInitialMainLocateOverride(resourceId: string): void {
  mainLocateOverride = resourceId
}

export function consumeInitialMainLocateOverride(): string | null {
  const resourceId = mainLocateOverride
  mainLocateOverride = null
  return resourceId
}

export function resetInitialMainLocateOverride(): void {
  mainLocateOverride = null
}
