import { serverErrorSchema } from '@/type/schemas'
import axios from 'axios'

export function errorDisplay(error: unknown): string {
  if (axios.isAxiosError(error)) {
    console.error('Axios error:', error)

    const message = error.message

    const parsed = serverErrorSchema.safeParse(error.response?.data)
    if (parsed.success) {
      return `${message}: ${parsed.data.error}`
    }

    return message
  }

  if (error instanceof Error) return error.message

  return 'Unknown error occurred'
}
