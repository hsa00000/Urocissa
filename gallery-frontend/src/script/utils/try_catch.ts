import { useMessageStore } from '@/store/messageStore'
import { errorDisplay } from './errorDisplay'
import type { IsolationId } from '@type/types'

/**
 * Utility function to handle try-catch with automatic error handling using messageStore
 * @param tryFn - The function to execute in the try block
 * @param isolationId - The isolation ID for the message store (defaults to 'mainId')
 * @returns Promise<T> - Returns the result of tryFn if successful, undefined if error occurs
 */
export async function tryWithMessageStore<T>(
  isolationId: IsolationId = 'mainId',
  tryFn: () => Promise<T>
): Promise<T | undefined> {
  const messageStore = useMessageStore(isolationId)

  try {
    return await tryFn()
  } catch (error: unknown) {
    messageStore.error(errorDisplay(error))
    return undefined
  }
}

/**
 * Synchronous version of tryWithMessageStore
 * @param isolationId - The isolation ID for the message store (defaults to 'mainId')
 * @param tryFn - The function to execute in the try block
 * @returns T | undefined - Returns the result of tryFn if successful, undefined if error occurs
 */
export function tryWithMessageStoreSync<T>(
  isolationId: IsolationId = 'mainId',
  tryFn: () => T
): T | undefined {
  const messageStore = useMessageStore(isolationId)

  try {
    return tryFn()
  } catch (error: unknown) {
    messageStore.error(errorDisplay(error))
    return undefined
  }
}
