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
  PublicConfigSchema,
  prefetchReturnSchema,
  tokenReturnSchema,
  ShareSchema
} from '@type/schemas'

export type Sorting = 'ascending' | 'descending' | 'random' | 'similar'

export interface SlicedDataItem {
  index: number
  data: AbstractData
}

export interface SlicedData {
  index: number
  data: AbstractData
}

export interface Message {
  text: string
  color: MessageColor
}

export type Alias = z.infer<typeof AliasSchema>
export type TagInfo = z.infer<typeof tagInfoSchema>
export type AlbumInfo = z.infer<typeof albumInfoSchema>
export type ScrollbarData = z.infer<typeof scrollbarDataSchema>
export type DisplayElement = z.infer<typeof displayElementSchema>
export type Row = z.infer<typeof rowSchema>
export type RowWithOffset = z.infer<typeof rowWithOffsetSchema>
export type Prefetch = z.infer<typeof prefetchSchema>
export type PrefetchReturn = z.infer<typeof prefetchReturnSchema>
export type Album = z.infer<typeof AlbumSchema>
export type Database = z.infer<typeof DataBaseSchema>
export type AbstractData = z.infer<typeof AbstractDataSchema>
export type SubRow = z.infer<typeof SubRowSchema>
export type PublicConfig = z.infer<typeof PublicConfigSchema>
export type IsolationId = 'mainId' | 'subId' | 'tempId' | 'shareId'
export type FetchDataMethod = 'batch' | 'single'
export type TokenReturn = z.infer<typeof tokenReturnSchema>
export type MessageColor = 'error' | 'success' | 'info'
export type Share = z.infer<typeof ShareSchema>
