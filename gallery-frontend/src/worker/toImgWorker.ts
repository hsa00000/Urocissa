import { readAndCompressImage } from '@misskey-dev/browser-image-resizer'
import { bindActionDispatch, createHandler } from 'typesafe-agent-events'
import { fromImgWorker, toImgWorker } from './workerApi'
import type {
  processAbortPayload,
  processImagePayload,
  processSmallImagePayload
} from './workerApi'
import axiosRetry from 'axios-retry'
import axios, { AxiosError } from 'axios'
import { getSrc } from '@/../config'

const controllerMap = new Map<number, AbortController>()

axiosRetry(axios, {
  retries: Infinity,
  retryDelay: () => {
    return 200
  },
  retryCondition: (error) => {
    // Check if the error is a cancellation
    if (axios.isCancel(error)) {
      return false // Do not retry
    }

    // Check if the error is an AxiosError and has a response property
    const response = (error as AxiosError).response
    if (response) {
      return response.status !== 200
    }

    // Handle other unknown situations
    return true
  }
})

const postToMain = bindActionDispatch(fromImgWorker, self.postMessage.bind(self))
const handler = createHandler<typeof toImgWorker>({
  async processSmallImage(event: processSmallImagePayload) {
    try {
      const controller = new AbortController()
      controllerMap.set(event.index, controller)

      console.log('using `Bearer ${event.token}`', `Bearer ${event.token}`)

      const response = await axios.get<Blob>(
        getSrc(event.hash, false, 'jpg', event.jwt, undefined),
        {
          signal: controller.signal,
          responseType: 'blob',
          headers: {
            Authorization: `Bearer ${event.token}` // or another authentication scheme
          }
        }
      )
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
      postToMain.smallImageProcessed({ index: event.index, url: objectUrl })
    } catch (error) {
      if (axios.isCancel(error)) {
        // Do nothing if the error is due to cancellation
        return
      }
      console.error(error)
    }
  },
  async processImage(event: processImagePayload) {
    try {
      const response = await axios.get<Blob>(
        getSrc(event.hash, false, 'jpg', event.jwt, undefined),
        {
          responseType: 'blob',
          headers: {
            Authorization: `Bearer ${event.token}` // or another authentication scheme
          }
        }
      )
      const blob = response.data
      const img = await createImageBitmap(blob)

      // Create an OffscreenCanvas
      const offscreenCanvas = new OffscreenCanvas(img.width, img.height)
      const context = offscreenCanvas.getContext('2d')
      context?.drawImage(img, 0, 0)

      // Convert the canvas to a blob
      const orientedImgBlob = await offscreenCanvas.convertToBlob()
      const objectUrl = URL.createObjectURL(orientedImgBlob)

      postToMain.imageProcessed({ index: event.index, url: objectUrl })
    } catch (error) {
      console.error(error)
    }
  },
  processAbort(event: processAbortPayload) {
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
