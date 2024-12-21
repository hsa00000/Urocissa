// types.ts

import { z } from 'zod'
import {
  AliasSchema,
  tagInfoSchema,
  scrollbarDataSchema,
  displayElementSchema,
  rowSchema,
  rowWithOffsetSchema,
  prefetchSchema,
  AlbumSchema,
  DataBaseSchema,
  AbstractDataSchema,
  SubRowSchema,
  albumInfoSchema,
  PublicConfigSchema
} from './schemas'

/**
 * Sorting options.
 */
export type Sorting = 'ascending' | 'descending' | 'random' | 'similar'

/**
 * Represents a sliced data item with an index and AbstractData instance.
 */
export interface SlicedDataItem {
  index: number
  data: AbstractData
}

/**
 * Represents sliced data with an index and AbstractData instance.
 */
export interface SlicedData {
  index: number
  data: AbstractData
}

/**
 * Represents prefetching sliced data.
 */
export interface PreFetchSliced {
  timestamp: string
  locateTo?: number
  dataLength: number
}

/**
 * Represents an alias with file information.
 */
export type Alias = z.infer<typeof AliasSchema>

/**
 * Represents tag information.
 */
export type TagInfo = z.infer<typeof tagInfoSchema>

export type AlbumInfo = z.infer<typeof albumInfoSchema>

/**
 * Represents scrollbar data.
 */
export type ScrollbarData = z.infer<typeof scrollbarDataSchema>

/**
 * Represents display elements within a row.
 */
export type DisplayElement = z.infer<typeof displayElementSchema>

/**
 * Represents a row within the layout.
 */
export type Row = z.infer<typeof rowSchema>

/**
 * Represents a row with offset information.
 */
export type RowWithOffset = z.infer<typeof rowWithOffsetSchema>

/**
 * Represents prefetch information.
 */
export type Prefetch = z.infer<typeof prefetchSchema>

/**
 * Represents Album.
 */
export type Album = z.infer<typeof AlbumSchema>

/**
 * Represents DataBase.
 */
export type DataBase = z.infer<typeof DataBaseSchema>

/**
 * Represents AbstractData.
 */
export type AbstractData = z.infer<typeof AbstractDataSchema>

/**
 * Represents SubRow.
 */
export type SubRow = z.infer<typeof SubRowSchema>

export type PublicConfig = z.infer<typeof PublicConfigSchema>

export type IsolationId = 'mainId' | 'subId' | 'tempId'
