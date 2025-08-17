import { rowSchema, rowWithOffsetSchema, databaseTimestampSchema } from '@type/schemas'
import {
  AbstractData,
  Database,
  DisplayElement,
  FetchDataMethod,
  Row,
  RowWithOffset,
  SlicedData,
  SubRow
} from '@type/types'
import { batchNumber, fixedBigRowHeight, paddingPixel } from '@/type/constants'
import { getArrayValue } from '@utils/getter'
import { createAbstractData, createAlbum, createDataBase } from '@utils/createData'

import axios from 'axios'

import { bindActionDispatch, createHandler } from 'typesafe-agent-events'
import { fromDataWorker, toDataWorker } from './workerApi'
import { z } from 'zod'

const shouldProcessBatch: number[] = []
const fetchedRowData = new Map<number, Row>()
const postToMainData = bindActionDispatch(fromDataWorker, self.postMessage.bind(self))
const workerAxios = axios.create()

self.addEventListener('message', (e) => {
  const handler = createHandler<typeof toDataWorker>({
    fetchData: async (payload) => {
      const { fetchMethod, batch, timestamp, timestampToken } = payload

      // if there are too many batch are processed then try to terminate the oldest request
      if (fetchMethod === 'batch') {
        if (shouldProcessBatch.length >= 6) {
          shouldProcessBatch.shift()
        }
        shouldProcessBatch.push(batch)
      }

      const { result, startIndex, endIndex } = await fetchData(
        fetchMethod,
        batch,
        timestamp,
        timestampToken
      )

      if (result.size > 0) {
        const indices = Array.from({ length: endIndex - startIndex }, (_, i) => startIndex + i)

        //Push the result Map into a SlicedData[]
        const slicedDataArray: SlicedData[] = []
        for (const index of indices) {
          const getData = result.get(index)
          if (getData !== undefined) {
            slicedDataArray.push({
              index,
              data: getData.abstractData,
              hashToken: getData.hashToken
            })
          }
        }

        postToMainData.returnData({ batch: batch, slicedDataArray: slicedDataArray })
      }
    },
    fetchRow: async (payload) => {
      const {
        index,
        timestamp,
        windowWidth,
        isLastRow,
        timestampToken,
        subRowHeightScale,
        limitRatio
      } = payload

      const rowWithOffset = await fetchRow(
        index,
        timestamp,
        windowWidth,
        isLastRow,
        timestampToken,
        subRowHeightScale,
        limitRatio
      )

      postToMainData.fetchRowReturn({
        rowWithOffset,
        timestamp,
        subRowHeightScale
      })
    }
  })
  handler(e.data as ReturnType<(typeof toDataWorker)[keyof typeof toDataWorker]>)
})

/**
 * Fetches a batch of data based on the provided batch index and timestamp.
 * Processes the fetched data into Database instances and accumulates them into a map.
 *
 * @param batchIndex - The index of the batch to fetch.
 * @param timestamp - The timestamp associated with the data fetch.
 * @returns A promise that resolves to a Map of data entries keyed by their index,
 *          or an object containing an error message and a warning flag if an error occurs.
 */
async function fetchData(
  fetchMethod: FetchDataMethod,
  index: number,
  timestamp: number,
  timestampToken: string
): Promise<{
  result: Map<number, { abstractData: AbstractData; hashToken: string }>
  startIndex: number
  endIndex: number
}> {
  let start: number
  let end: number
  switch (fetchMethod) {
    case 'batch': {
      const batchIndex = index
      start = batchIndex * batchNumber
      end = (batchIndex + 1) * batchNumber
      break
    }
    case 'single': {
      start = index
      end = index + 1
      break
    }
  }

  const fetchUrl = `/get/get-data?timestamp=${timestamp}&start=${start}&end=${end}`

  const response = await workerAxios.get<Database[]>(fetchUrl, {
    headers: {
      Authorization: `Bearer ${timestampToken}`
    }
  })
  const databaseTimestampArray = z.array(databaseTimestampSchema).parse(response.data)

  const data = new Map<number, { abstractData: AbstractData; hashToken: string }>()

  for (let i = 0; i < databaseTimestampArray.length; i++) {
    // Determine the current batch index based on the fetch method
    const currentBatchIndex = fetchMethod === 'batch' ? Math.floor(start / batchNumber) : index

    if (fetchMethod === 'batch' && !shouldProcessBatch.includes(currentBatchIndex)) {
      break // Stop processing further if the batch should no longer be processed
    }

    const item = databaseTimestampArray[i]
    const key = start + i

    if (item === undefined) {
      console.error(
        `Error processing item at ${fetchMethod === 'batch' ? 'batchIndex' : 'index'}: ${
          fetchMethod === 'batch' ? index : index
        }, ` + `batchNumber: ${batchNumber}, index: ${i}. Item is undefined.`
      )
      continue
    }

    if ('Database' in item.abstractData) {
      const databaseInstance = createDataBase(item.abstractData.Database, item.timestamp)
      const abstractData = createAbstractData(databaseInstance)
      data.set(key, { abstractData, hashToken: item.token })
    } else if ('Album' in item.abstractData) {
      const albumInstance = createAlbum(item.abstractData.Album, item.timestamp)
      const abstractData = createAbstractData(albumInstance)
      data.set(key, { abstractData, hashToken: item.token })
    }

    if (i % 100 === 0) {
      // Yield after every 100 items
      await new Promise((resolve) => setTimeout(resolve, 0))
    }
  }

  return { result: data, startIndex: start, endIndex: end }
}

/**
 * Fetches a specific row of display elements based on the provided index and timestamp,
 * applies layout algorithms to arrange the elements within the specified window width,
 * and returns the row along with its offset.
 *
 * @param index - The index of the row to fetch.
 * @param timestamp - The timestamp associated with the data fetch.
 * @param windowWidth - The width of the window/container.
 * @returns A promise that resolves to a RowWithOffset object containing the row data and its offset,
 *          or undefined if an error occurs during fetching or parsing.
 */
async function fetchRow(
  index: number,
  timestamp: number,
  windowWidth: number,
  isLastRow: boolean,
  timestampToken: string,
  subRowHeightScale: number,
  limitRatio: boolean
): Promise<RowWithOffset> {
  // Retrieve the cached row (if any) but always operate on a clone
  let cachedRow = fetchedRowData.get(index)

  if (cachedRow === undefined) {
    const response = await workerAxios.get<Row>(
      `/get/get-rows?index=${index}&timestamp=${timestamp}&window_width=${Math.round(windowWidth)}`,
      {
        headers: {
          Authorization: `Bearer ${timestampToken}`
        }
      }
    )
    cachedRow = rowSchema.parse(response.data)
    // store a clone so the stored value remains immutable for future reuse
    fetchedRowData.set(cachedRow.rowIndex, structuredClone(cachedRow))
  }

  // Work on a fresh clone to avoid mutating the cached reference
  const row = structuredClone(cachedRow)

  if (limitRatio) {
    for (const displayElement of row.displayElements) {
      if (displayElement.displayWidth > displayElement.displayHeight * 2) {
        displayElement.displayWidth = displayElement.displayHeight * 2
      } else if (displayElement.displayHeight > displayElement.displayWidth * 2) {
        displayElement.displayHeight = displayElement.displayWidth * 2
      }
    }
  }

  // Setting row.topPixelAccumulated
  row.topPixelAccumulated = row.rowIndex * fixedBigRowHeight

  const subRowHeight = Math.round(Math.min(windowWidth / 2, subRowHeightScale))

  // Perform Algorithm to wrap row into subrows
  const subRows = KnuthPlassLayout(row, windowWidth, subRowHeight)

  // Normalize subrows by scaling their widths and heights to fit within the window width
  const scaledTotalHeight = normalizeSubrows(subRows, windowWidth, isLastRow, subRowHeight)

  // Setting row.rowHeight
  row.rowHeight = scaledTotalHeight

  // Calculate the offset, which represents the difference between the original default height and the actual wrapped height
  const offset = scaledTotalHeight - fixedBigRowHeight

  const rowWithOffset = rowWithOffsetSchema.parse({
    row: row,
    offset: offset,
    windowWidth
  })

  return rowWithOffset
}

/**
 * Wraps elements into lines based on their widths and the window width,
 * minimizing the badness (unused space squared) of each line.
 *
 * @param displayElements - Array of element widths.
 * @param windowWidth - The width of the window/container.
 * @returns An array where each element represents the number of items in a line.
 */
function lineWrap(displayElements: number[], windowWidth: number): number[] {
  const n = displayElements.length
  const minBadness = Array<number>(n + 1).fill(Infinity)
  const breaks = Array<number>(n + 1).fill(0)

  minBadness[0] = 0

  for (let i = 1; i <= n; i++) {
    let currentWidth = 0
    for (let j = i; j > 0; j--) {
      currentWidth += getArrayValue(displayElements, j - 1) + 2 * paddingPixel

      if (currentWidth > windowWidth) break

      const badness = Math.pow(windowWidth - currentWidth, 2)
      if (getArrayValue(minBadness, j - 1) + badness < getArrayValue(minBadness, i)) {
        minBadness[i] = getArrayValue(minBadness, j - 1) + badness
        breaks[i] = j - 1
      }
    }

    // Check if a valid line break was found. If not, start a new line with the current element.
    if (minBadness[i] === Infinity) {
      minBadness[i] = getArrayValue(minBadness, i - 1) + Math.pow(windowWidth - currentWidth, 2)
      breaks[i] = i - 1
    }
  }

  const lineCounts: number[] = []
  let end = n
  while (end > 0) {
    const start = getArrayValue(breaks, end)
    lineCounts.unshift(end - start) // Calculate the number of elements in each line
    end = start
  }

  return lineCounts
}

/**
 * Arranges display elements into subrows using the Knuth-Plass layout algorithm.
 *
 * @param row - The row containing display elements to layout.
 * @param windowWidth - The width of the window/container.
 * @returns An array of SubRow objects representing the layout.
 */
function KnuthPlassLayout(row: Row, windowWidth: number, subRowHeight: number): SubRow[] {
  // Calculate the list of widths after scaling based on the subrow height
  const shrinkedWidthList = row.displayElements.map((displayElement) => {
    return Math.round((displayElement.displayWidth * subRowHeight) / displayElement.displayHeight)
  })

  // Use the lineWrap function to determine how many elements fit in each line
  const lineWrapBatchResult = lineWrap(shrinkedWidthList, windowWidth)

  const allDisplayElement: DisplayElement[] = row.displayElements
  const result: SubRow[] = []

  let startIndex = 0

  // Split allDisplayElement into subrows based on lineWrapBatchResult
  for (const count of lineWrapBatchResult) {
    const subArray = allDisplayElement.slice(startIndex, startIndex + count)
    const subrow: SubRow = {
      displayElements: subArray
    }
    result.push(subrow)
    startIndex += count
  }

  return result
}

/**
 * Normalize subrows by scaling their widths and heights to fit within the window width.
 *
 * @param subRows - Array of subrows to normalize.
 * @param windowWidth - The width of the window/container.
 * @param paddingLastSubrow - Whether to skip scaling logic for the last subrow.
 * @returns The total scaled height of all subrows.
 */
function normalizeSubrows(
  subRows: SubRow[],
  windowWidth: number,
  paddingLastSubrow: boolean,
  subRowHeight: number
): number {
  let scaledTotalHeight = 0
  let displayTopPixelAccumulated = 0

  subRows.forEach((subRow, rowIndex) => {
    let widthSum = 0

    // Adjust for the last subrow if paddingLastSubrow is true
    const isLastSubrow = paddingLastSubrow && rowIndex === subRows.length - 1
    // Scale elements in the subrow
    subRow.displayElements.forEach((displayElement) => {
      const width = displayElement.displayWidth
      const height = displayElement.displayHeight

      const scaledWidth = (width * subRowHeight) / height
      displayElement.displayWidth = scaledWidth
      displayElement.displayHeight = subRowHeight
    })

    if (!isLastSubrow) {
      // Calculate total width of elements in the subrow
      widthSum = subRow.displayElements.reduce(
        (sum, displayElement) => sum + displayElement.displayWidth,
        0
      )

      const ratio = (windowWidth - subRow.displayElements.length * 2 * paddingPixel) / widthSum
      const scaledHeight = subRowHeight * ratio

      // Adjust elements' width and height based on the ratio
      widthSum = paddingPixel // Reset width sum with initial padding
      subRow.displayElements.forEach((displayElement, index) => {
        if (index < subRow.displayElements.length - 1) {
          displayElement.displayWidth = Math.round(displayElement.displayWidth * ratio)
          displayElement.displayHeight = Math.round(scaledHeight)
          widthSum += displayElement.displayWidth + 2 * paddingPixel
        } else {
          displayElement.displayWidth = windowWidth - widthSum - paddingPixel
          displayElement.displayHeight = Math.round(scaledHeight)
        }
        displayElement.displayTopPixelAccumulated = displayTopPixelAccumulated
      })

      displayTopPixelAccumulated += Math.round(scaledHeight + 2 * paddingPixel)
      scaledTotalHeight += Math.round(scaledHeight + 2 * paddingPixel)
    } else {
      // Skip scaling logic, use fixed subRowHeight for the last subrow
      subRow.displayElements.forEach((displayElement) => {
        displayElement.displayHeight = subRowHeight
        displayElement.displayTopPixelAccumulated = displayTopPixelAccumulated
      })

      displayTopPixelAccumulated += subRowHeight + 2 * paddingPixel
      scaledTotalHeight += subRowHeight + 2 * paddingPixel
    }
  })

  return scaledTotalHeight
}
