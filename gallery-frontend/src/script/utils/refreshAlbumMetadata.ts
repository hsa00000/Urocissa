import { fetchDataInWorker } from '@/script/inWorker/fetchDataInWorker'
import { useDataStore } from '@/store/dataStore'
import { useMessageStore } from '@/store/messageStore'
import { useWorkerStore } from '@/store/workerStore'
import { bindActionDispatch } from 'typesafe-agent-events'
import { toImgWorker } from '@/worker/workerApi'
import { getCookiesJwt } from '@utils/getter'
import { watch } from 'vue'
export function refreshAlbumMetadata(albumId: string) {
  const dataStore = useDataStore('mainId')
  const workerStore = useWorkerStore('mainId')
  const messageStore = useMessageStore('mainId')

  const albumIndex = dataStore.hashMapData.get(albumId)
  if (albumIndex === undefined) {
    console.error(`cannot find albumIndex with albumId = ${albumId}`)
    return
  }

  // perform after fetchDataInWorker
  const stopWatch = watch(
    () => dataStore.data.get(albumIndex),
    () => {
      const postToWorker = bindActionDispatch(toImgWorker, (action) => {
        const worker = workerStore.imgWorker[0]
        if (worker) {
          worker.postMessage(action)
        } else {
          throw new Error(`Worker not found for index: 0`)
        }
      })

      const album = dataStore.data.get(albumIndex)?.album

      if (!album) {
        console.error(`cannot find album with albumIndex = ${albumIndex}`)
        return
      }

      const coverHash = album.cover

      if (coverHash === null) {
        return
      }

      postToWorker.processImage({
        index: albumIndex,
        hash: coverHash,
        devicePixelRatio: window.devicePixelRatio,
        jwt: getCookiesJwt(),
        token: ''
      })

      postToWorker.processSmallImage({
        index: albumIndex,
        hash: coverHash,
        width: 300,
        height: 300,
        devicePixelRatio: window.devicePixelRatio,
        jwt: getCookiesJwt(),
        albumMode: true,
        token: ''
      })

      messageStore.showInfo('Edit successfully.')
      stopWatch()
    }
  )

  fetchDataInWorker('single', albumIndex, 'mainId')
}
