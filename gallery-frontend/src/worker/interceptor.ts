import axios, { AxiosInstance, AxiosResponse } from 'axios'
import { tokenReturnSchema } from '@/script/common/schemas'
import { postToMain } from './toDataWorker'
import { storeToken } from '@/indexedDb/token'

export function setupAxiosInterceptors(axiosInstance: AxiosInstance): void {
  axiosInstance.interceptors.response.use(
    (response: AxiosResponse) => response,
    async (error) => {
      if (!axios.isAxiosError(error)) {
        console.error('Unexpected error:', error)
        postToMain.notification({ message: 'An unexpected error occurred', messageType: 'warn' })
        console.error(error)
        return Promise.reject(error instanceof Error ? error : new Error(String(error)))
      }

      const { config, response } = error

      if (response?.status === 401) {
        const requestUrl = config?.url

        // Check if the request URL matches any of the specified endpoints
        if (requestUrl === undefined) {
          postToMain.unauthorized()
          return
        }
        if (
          requestUrl.startsWith('/get/get-data') ||
          requestUrl.startsWith('/get/get-rows') ||
          requestUrl.startsWith('/get/get-scroll-bar')
        ) {
          try {
            const authHeader = config?.headers.Authorization
            const expiredToken =
              typeof authHeader === 'string' && authHeader.startsWith('Bearer ')
                ? authHeader.split(' ')[1]
                : null

            if (expiredToken == null) {
              throw new Error('No expired token found')
            }

            const tokenResponse = await axios.post('/post/renew-timestamp-token', {
              token: expiredToken
            })

            if (tokenResponse.status === 200) {
              const newToken = tokenReturnSchema.parse(tokenResponse.data)
              if (config) {
                config.headers.Authorization = `Bearer ${newToken.token}`
                postToMain.renewTimestampToken({ token: newToken.token })
                await storeToken(newToken.token)
                return await axios.request(config)
              }
            }
          } catch (err) {
            console.error('Token renewal failed:', err)
          }
        } else {
          postToMain.unauthorized()
        }
        postToMain.notification({ message: 'Unauthorized. Please log in.', messageType: 'warn' })
      } else if (response) {
        postToMain.notification({ message: 'An error occurred', messageType: 'warn' })
      } else {
        postToMain.notification({ message: 'No response from server', messageType: 'warn' })
      }

      console.error(error)
      return Promise.reject(error)
    }
  )
}
