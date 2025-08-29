import { AxiosInstance, AxiosError } from 'axios'
import { serverErrorSchema } from '@type/schemas'

export function setupAxiosInterceptor(
  axiosInstance: AxiosInstance,
  notify: (payload: { text: string; color: 'error' }) => void
) {
  axiosInstance.interceptors.response.use(
    (response) => response,
    (error: AxiosError) => {
      if (error.response?.status === 500) {
        let errorMessage: string

        try {
          const parsedError = serverErrorSchema.parse(error.response.data)
          errorMessage = parsedError.error
        } catch (parseError) {
          console.error('Failed to parse server error response:', parseError)
          errorMessage = 'Unknown error occurred'
        }

        notify({ text: errorMessage, color: 'error' })
      }
      return Promise.reject(error)
    }
  )
}