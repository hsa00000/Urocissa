import { IsolationId } from '@type/types'
import { handleDataWorkerReturn, removeHandleDataWorkerReturn } from '@/worker/fromDataWorker'
import { handleImgWorker, removeHandleImgWorkerReturn } from '@/worker/fromImgWorker'
import { PostToDataWorker, PostToImgWorker, toDataWorker, toImgWorker } from '@/worker/workerApi'
import { defineStore } from 'pinia'
import { bindActionDispatch } from 'typesafe-agent-events'
import { useConstStore } from './constStore'

export const useWorkerStore = (isolationId: IsolationId) =>
  defineStore('workerStore' + isolationId, {
    state: (): {
      worker: null | Worker
      imgWorker: Worker[]
      postToDataWorker: PostToDataWorker | undefined
      postToImgWorkerList: PostToImgWorker[] | undefined
    } => ({
      worker: null,
      imgWorker: [],
      postToDataWorker: undefined,
      postToImgWorkerList: undefined
    }),
    actions: {
      initializeWorker(isolationId: IsolationId) {
        if (this.worker === null) {
          this.worker = new Worker(new URL('../worker/toDataWorker.ts', import.meta.url), {
            type: 'module'
          })
          handleDataWorkerReturn(this.worker, isolationId)
          this.postToDataWorker = bindActionDispatch(toDataWorker, (action) => {
            this.worker?.postMessage(action)
          })
        } else {
          console.error('There is already a worker')
        }

        if (this.imgWorker.length === 0) {
          const constStore = useConstStore('mainId')
          this.postToImgWorkerList = []
          for (let i = 0; i <= constStore.concurrencyNumber; i++) {
            const worker = new Worker(new URL('../worker/toImgWorker.ts', import.meta.url), {
              type: 'module'
            })
            this.imgWorker.push(worker)
            const postToDataWorker = bindActionDispatch(toImgWorker, (action) => {
              worker.postMessage(action)
            })
            this.postToImgWorkerList.push(postToDataWorker)
          }
          this.imgWorker.forEach((worker) => {
            handleImgWorker(worker, isolationId)
          })
        } else {
          console.error('There is already an imgWorker')
        }
      },
      terminateWorker() {
        if (this.worker !== null) {
          this.worker.terminate()
          removeHandleDataWorkerReturn(this.worker)
          this.worker = null
        } else {
          console.error('No Worker is Working')
        }
        if (this.imgWorker.length > 0) {
          this.imgWorker.forEach((worker) => {
            worker.terminate()
            removeHandleImgWorkerReturn(worker)
          })
          this.imgWorker = []
        } else {
          console.error('No Worker is Working')
        }
      }
    }
  })()
