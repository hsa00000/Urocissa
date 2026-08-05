import { serverErrorSchema } from '@/type/schemas'
import axios from 'axios'

export const STALE_GENERATION_REFRESH_MESSAGE =
  'The selection is out of date. Please refresh the page and try again.'

export function errorDisplay(error: unknown): string {
  if (axios.isAxiosError(error)) {
    console.error('Axios error:', error)

    const message = error.message

    const parsed = serverErrorSchema.safeParse(error.response?.data)
    if (parsed.success) {
      // Prefer 'message' from AppError, fallback to 'error' from legacy/other
      const serverMsg = parsed.data.message ?? parsed.data.error
      if (serverMsg != null && serverMsg.length > 0) {
        if (serverMsg.toLowerCase().includes('stale generation')) {
          return STALE_GENERATION_REFRESH_MESSAGE
        }
        return serverMsg
      }
    }

    return message
  }

  if (error instanceof Error) return error.message

  return 'Unknown error occurred'
}
