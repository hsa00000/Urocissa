import { readAndCompressImage } from '@misskey-dev/browser-image-resizer'
import { bindActionDispatch, createHandler } from 'typesafe-agent-events'
import { fromImgWorker, toImgWorker } from '@/worker/workerApi'
import {
  ProcessAbortPayload,
  ProcessImagePayload,
  ProcessSmallImagePayload
} from '@/worker/workerApi'
import axiosRetry from 'axios-retry'
import axios, { AxiosError } from 'axios'
import { getSrc } from '@utils/getter'
import { setupAxiosInterceptor } from './axiosInterceptor'

const postToMainImg = bindActionDispatch(fromImgWorker, self.postMessage.bind(self))
const controllerMap = new Map<number, AbortController>()
const workerAxios = axios.create()

axiosRetry(workerAxios, {
  retries: 0,
  retryDelay: () => 200,
  retryCondition: (error) => {
    if (axios.isCancel(error)) return false
    const response = (error as AxiosError).response
    return response ? response.status !== 200 : true
  }
})

setupAxiosInterceptor(workerAxios, postToMainImg.notification)

const handler = createHandler<typeof toImgWorker>({
  async processSmallImage(event: ProcessSmallImagePayload) {
    try {
      const controller = new AbortController()
      controllerMap.set(event.index, controller)

      const headers: Record<string, string> = {}
      if (event.albumId !== null) headers['x-album-id'] = event.albumId
      if (event.shareId !== null) headers['x-share-id'] = event.shareId

      headers.Authorization = `Bearer ${event.hashToken}`

      const config = {
        signal: controller.signal,
        responseType: 'blob' as const,
        headers,
        timestampToken: event.timestampToken
      }

      const response = await workerAxios.get<Blob>(getSrc(event.hash, false, 'jpg'), config)

      controllerMap.delete(event.index)
      const blob = response.data
      const img = await createImageBitmap(blob)

      const albumMode = event.albumMode === true
      const converted: Blob = await readAndCompressImage(img, {
        argorithm: 'bilinear',
        quality: 1,
        maxWidth: albumMode
          ? img.width *
            Math.max(event.width / img.width, event.height / img.height) *
            event.devicePixelRatio
          : event.width * event.devicePixelRatio,
        maxHeight: albumMode
          ? img.height *
            Math.max(event.width / img.width, event.height / img.height) *
            event.devicePixelRatio
          : event.height * event.devicePixelRatio
      })

      const objectUrl = URL.createObjectURL(converted)
      postToMainImg.smallImageProcessed({ index: event.index, url: objectUrl })
    } catch (error) {
      if (axios.isCancel(error)) return
      console.error(error)
    }
  },

  async processImage(event: ProcessImagePayload) {
    try {
      const headers: Record<string, string> = {}
      if (event.albumId !== null) headers['x-album-id'] = event.albumId
      if (event.shareId !== null) headers['x-share-id'] = event.shareId
      headers.Authorization = `Bearer ${event.hashToken}`

      const config = {
        responseType: 'blob' as const,
        headers,
        timestampToken: event.timestampToken
      }

      const response = await workerAxios.get<Blob>(getSrc(event.hash, false, 'jpg'), config)
      const blob = response.data
      const img = await createImageBitmap(blob)

      const offscreenCanvas = new OffscreenCanvas(img.width, img.height)
      const context = offscreenCanvas.getContext('2d')
      context?.drawImage(img, 0, 0)

      const orientedImgBlob = await offscreenCanvas.convertToBlob()
      const objectUrl = URL.createObjectURL(orientedImgBlob)

      postToMainImg.imageProcessed({ index: event.index, url: objectUrl })
    } catch (error) {
      console.error(error)
    }
  },

  processAbort(event: ProcessAbortPayload) {
    const controller = controllerMap.get(event.index)
    if (controller !== undefined) {
      controller.abort()
      controllerMap.delete(event.index)
    }
  }
})

self.addEventListener('message', (e) => {
  handler(e.data as ReturnType<(typeof toImgWorker)[keyof typeof toImgWorker]>)
})
