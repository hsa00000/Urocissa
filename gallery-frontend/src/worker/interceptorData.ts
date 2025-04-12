/* eslint-disable @typescript-eslint/return-await */
import axios, { AxiosInstance, AxiosResponse, InternalAxiosRequestConfig } from 'axios'
import { tokenReturnSchema } from '@type/schemas'
import { storeTimestampToken } from '@/indexedDb/timestampToken'
import { PostToMainDataType } from './workerApi'

interface QueuedRequest {
  config: InternalAxiosRequestConfig | undefined
  resolve: (value: AxiosResponse) => void
  reject: (error: unknown) => void
}

let isRefreshing = false
const queueList: QueuedRequest[] = []

export function interceptorData(
  axiosInstance: AxiosInstance,
  postToMainData: PostToMainDataType
): void {
  axiosInstance.interceptors.response.use(
    (response: AxiosResponse) => response,
    async (error) => {
      if (!axios.isAxiosError(error)) {
        console.error('Unexpected error:', error)
        postToMainData.notification({
          text: 'An unexpected error occurred',
          color: 'error'
        })
        return Promise.reject(error instanceof Error ? error : new Error(String(error)))
      }

      const { config, response } = error

      if (response?.status === 401) {
        const requestUrl = config?.url

        if (requestUrl == null) {
          postToMainData.unauthorized()
          return Promise.reject(error)
        }

        if (
          requestUrl.startsWith('/get/get-data') ||
          requestUrl.startsWith('/get/get-rows') ||
          requestUrl.startsWith('/get/get-scroll-bar') ||
          requestUrl.startsWith('/post/renew-hash-token')
        ) {
          const authHeader = config?.headers.Authorization
          const expiredToken =
            typeof authHeader === 'string' && authHeader.startsWith('Bearer ')
              ? authHeader.split(' ')[1]
              : null

          if (expiredToken == null) {
            return Promise.reject(new Error('No expired token found'))
          }

          if (isRefreshing) {
            // Return a promise that queues this retried request
            return new Promise((resolve, reject) => {
              queueList.push({ config, resolve, reject })
            })
          }

          isRefreshing = true

          try {
            const tokenResponse = await axios.post('/post/renew-timestamp-token', {
              token: expiredToken
            })

            if (tokenResponse.status === 200) {
              const newToken = tokenReturnSchema.parse(tokenResponse.data)
              await storeTimestampToken(newToken.token)

              // Update original failed request and retry
              if (config) {
                config.headers.Authorization = `Bearer ${newToken.token}`

                // Retry queued requests with the new token
                queueList.forEach((queued) => {
                  if (queued.config) {
                    queued.config.headers.Authorization = `Bearer ${newToken.token}`
                    axios
                      .request(queued.config)
                      .then((response) => {
                        // 請求成功時，執行 resolve，把 response 傳回去
                        queued.resolve(response)
                      })
                      .catch((error: unknown) => {
                        // 請求失敗時，執行 reject，把錯誤傳回去
                        queued.reject(error)
                      })
                  }
                })
                // Clear the queue
                queueList.length = 0

                return axios.request(config)
              }
            }
          } catch (err) {
            console.error('Token renewal failed:', err)
            // Reject all queued requests on error
            queueList.forEach((queued) => {
              queued.reject(err)
            })
            queueList.length = 0
          } finally {
            isRefreshing = false
          }
        } else {
          postToMainData.unauthorized()
        }
        postToMainData.notification({
          text: 'Unauthorized. Please log in.',
          color: 'error'
        })
      } else if (response) {
        postToMainData.notification({ text: 'An error occurred', color: 'error' })
      } else {
        postToMainData.notification({ text: 'No response from server', color: 'error' })
      }

      return Promise.reject(error)
    }
  )
}
