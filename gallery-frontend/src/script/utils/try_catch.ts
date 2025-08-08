import { useMessageStore } from '@/store/messageStore'
import { errorDisplay } from './errorDisplay'

/**
 * Utility function to handle try-catch with automatic error handling using messageStore
 * @param tryFn - The function to execute in the try block
 * @param isolationId - The isolation ID for the message store (defaults to 'mainId')
 * @returns Promise<T> - Returns the result of tryFn if successful, undefined if error occurs
 */
export async function tryWithMessageStore<T>(tryFn: () => Promise<T>): Promise<T | undefined> {
  const messageStore = useMessageStore()

  try {
    return await tryFn()
  } catch (error: unknown) {
    messageStore.error(errorDisplay(error))
    return undefined
  }
}
