/* eslint-disable @typescript-eslint/return-await */
import axios, { AxiosInstance, AxiosResponse } from 'axios'
import { tokenReturnSchema } from '@type/schemas'
import { postToMainImg } from './toImgWorker'
import { getTimestampToken } from '@/indexedDb/timestampToken'
import { interceptorData } from './interceptorData'
import { getHashToken, storeHashToken } from '@/indexedDb/hashToken'
import { extractHashFromPath } from '@/script/utils/getter'

const subAxios = axios.create()
interceptorData(subAxios)

export function interceptorImg(axiosInstance: AxiosInstance): void {
  axiosInstance.interceptors.response.use(
    (response: AxiosResponse) => response,
    async (error) => {
      if (!axios.isAxiosError(error)) {
        console.error('Unexpected error:', error)
        postToMainImg.notification({
          message: 'An unexpected error occurred',
          messageType: 'warn'
        })
        console.error(error)
        return Promise.reject(error instanceof Error ? error : new Error(String(error)))
      }

      const { config, response } = error

      if (response?.status === 401) {
        const requestUrl = config?.url

        // Check if the request URL matches any of the specified endpoints
        if (requestUrl === undefined) {
          postToMainImg.unauthorized()
          return
        }

        if (requestUrl.startsWith('/object')) {
          try {
            const hash = extractHashFromPath(requestUrl)

            if (hash === null) {
              throw new Error('Failed to extract hash from URL')
            }

            const expiredToken = await getHashToken(hash)

            if (expiredToken == null) {
              throw new Error('No hash token found in query parameters')
            }

            const timestampToken = await getTimestampToken()
            if (timestampToken === null) {
              throw new Error('No timestampToken found in query parameters')
            }

            const tokenResponse = await subAxios.post(
              `/post/renew-hash-token`,
              {
                expiredHashToken: expiredToken
              },
              {
                headers: {
                  Authorization: `Bearer ${timestampToken}`
                }
              }
            )

            if (tokenResponse.status === 200) {
              const newToken = tokenReturnSchema.parse(tokenResponse.data)

              await storeHashToken(hash, newToken.token)

              if (config) {
                return axiosInstance.request(config)
              }
            }
          } catch (err) {
            if (!(err instanceof Error && err.name === 'CanceledError')) {
              console.error('Token renewal failed:', err)
            }
          }
        } else {
          postToMainImg.unauthorized()
        }
        postToMainImg.notification({
          message: 'Unauthorized. Please log in.',
          messageType: 'warn'
        })
      }

      if (!(error instanceof Error && error.name === 'CanceledError')) {
        console.error('Token renewal failed:', error)
      }
      return Promise.reject(error)
    }
  )
}
