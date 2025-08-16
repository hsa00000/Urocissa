import {
  BrowserImageResizerConfigWithConvertedOutput,
  readAndCompressImage
} from '@misskey-dev/browser-image-resizer'
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

      const dpr = event.devicePixelRatio ?? 1
      const srcW = img.width,
        srcH = img.height
      const srcRatio = srcW / srcH

      const W = Math.max(1, Math.ceil(event.width * dpr))
      const H = Math.max(1, Math.ceil(event.height * dpr))

      const is2to1Wide = !!event.limitRatio && event.width === event.height * 2
      const is1to2Tall = !!event.limitRatio && event.height === event.width * 2

      const opts: Partial<BrowserImageResizerConfigWithConvertedOutput> = {
        argorithm: 'bilinear',
        quality: 1
      }

      // Default: contain to W×H (replaces the previous three duplicate branches)
      let targetW = W
      let targetH = H

      if (albumMode) {
        const scale = Math.max(event.width / srcW, event.height / srcH) * dpr
        targetW = Math.ceil(srcW * scale)
        targetH = Math.ceil(srcH * scale)
      } else if (is2to1Wide) {
        // Ultra-wide (exactly 2:1): height is primary, width scales proportionally
        targetH = H
        targetW = Math.ceil(targetH * srcRatio)
      } else if (is1to2Tall) {
        // Ultra-tall (exactly 1:2): width is primary, height scales proportionally
        targetW = W
        targetH = Math.ceil(targetW / srcRatio)
      }

      opts.maxWidth = targetW
      opts.maxHeight = targetH

      const converted: Blob = await readAndCompressImage(img, opts)

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
