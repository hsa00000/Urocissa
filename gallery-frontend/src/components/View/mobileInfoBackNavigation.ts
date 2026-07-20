interface MobileInfoBackNavigationOptions {
  isMobile: boolean
  isInfoOpen: boolean
  closeInfo: () => Promise<void>
  onCloseError: (error: unknown) => void
}

export async function interceptMobileInfoBackNavigation({
  isMobile,
  isInfoOpen,
  closeInfo,
  onCloseError
}: MobileInfoBackNavigationOptions): Promise<false | undefined> {
  if (!isMobile || !isInfoOpen) return

  try {
    await closeInfo()
  } catch (error: unknown) {
    onCloseError(error)
  }

  return false
}
