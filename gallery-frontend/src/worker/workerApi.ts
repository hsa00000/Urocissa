// Import necessary types from the commonType module
import { FetchDataMethod, RowWithOffset, SlicedData, TagInfo } from '@/script/common/types'

// Import createActionCreators from typesafe-agent-events for defining strongly-typed actions
import { createActionCreators } from 'typesafe-agent-events'

/**
 * Typesafe-agent-events is a library that helps create strongly-typed event-driven architectures.
 * It provides a utility called `createActionCreators` to define actions with type-safe payloads.
 * The resulting action creators can be used to send or receive messages between threads or workers.
 */

/* ================================================================================
 * toImgWorker
 * Actions to be sent to the image processing worker.
 * These actions are used to request image processing tasks.
 * ================================================================================ */

// Define payload types for image processing actions
export interface processSmallImagePayload {
  index: number
  hash: string
  width: number
  height: number
  devicePixelRatio: number
  jwt: string
  albumMode?: boolean
}

export interface processImagePayload {
  index: number
  hash: string
  devicePixelRatio: number
  jwt: string
}

export interface processAbortPayload {
  index: number
}

// Define actions to be sent to the image processing worker
export const toImgWorker = createActionCreators({
  processSmallImage: (payload: processSmallImagePayload) => payload,
  processImage: (payload: processImagePayload) => payload,
  processAbort: (payload: processAbortPayload) => payload
})

/* ================================================================================
 * fromImgWorker
 * Actions to be received from the image processing worker.
 * These actions are used to handle responses from the worker after processing images.
 * ================================================================================ */

// Define actions to be received from the image processing worker
export const fromImgWorker = createActionCreators({
  smallImageProcessed: (payload: { index: number; url: string }) => payload,
  imageProcessed: (payload: { index: number; url: string }) => payload,
  unauthorized: () => ({}),
  notification: (payload: NotificationReturnParams) => payload
})

/* ================================================================================
 * toDataWorker
 * Actions to be sent to the worker for various data and UI management tasks.
 * These actions request tasks like data fetching, resizing, and editing.
 * ================================================================================ */

// Define the parameter types for different actions to manage data and UI state
interface FetchDataParams {
  fetchMethod: FetchDataMethod
  batch: number
  timestamp: number
  timestampToken: string
}
interface FetchRowParams {
  index: number
  timestamp: number
  windowWidth: number
  isLastRow: boolean
  timestampToken: string
}
export interface EditTagsParams {
  indexSet: Set<number>
  addTagsArray: string[]
  removeTagsArray: string[]
  timestamp: number
}
export interface EditAlbumsParams {
  indexSet: Set<number>
  addAlbumsArray: string[]
  removeAlbumsArray: string[]
  timestamp: number
}
interface DeleteDataParams {
  indexArray: number[]
  timestamp: number
}

// Define actions for the worker to receive and execute tasks
export const toDataWorker = createActionCreators({
  fetchData: (payload: FetchDataParams) => payload,
  fetchRow: (payload: FetchRowParams) => payload,
  editTags: (payload: EditTagsParams) => payload,
  editAlbums: (payload: EditAlbumsParams) => payload,
  deleteData: (payload: DeleteDataParams) => payload
})

/* ================================================================================
 * fromDataWorker
 * Actions to be received from the worker after completing tasks.
 * These actions handle responses, data updates, or state changes.
 * ================================================================================ */

// Define the parameter types for responses from the worker back to the main thread
interface ReturnDataParams {
  batch: number
  slicedDataArray: SlicedData[]
}
interface FetchRowReturnParams {
  rowWithOffset: RowWithOffset
  timestamp: number
}
interface EditTagsReturnParams {
  returnedTagsArray: TagInfo[] | undefined
}
interface NotificationReturnParams {
  message: string
  messageType: 'info' | 'warn'
}

// Define actions for the worker to send responses back to the main thread
export const fromDataWorker = createActionCreators({
  returnData: (payload: ReturnDataParams) => payload,
  fetchRowReturn: (payload: FetchRowReturnParams) => payload,
  editTagsReturn: (payload: EditTagsReturnParams) => payload,
  notification: (payload: NotificationReturnParams) => payload,
  unauthorized: () => ({})
})
