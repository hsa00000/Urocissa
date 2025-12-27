import { z } from 'zod'
import {
  AliasSchema,
  tagInfoSchema,
  scrollbarDataSchema,
  displayElementSchema,
  rowSchema,
  rowWithOffsetSchema,
  prefetchSchema,
  SubRowSchema,
  albumInfoSchema,
  prefetchReturnSchema,
  ShareSchema,
  ResolvedShareSchema,
  TokenResponseSchema,
  BackendDataParser
} from '@type/schemas'

// 基礎類型
export type Alias = z.infer<typeof AliasSchema>
export type TagInfo = z.infer<typeof tagInfoSchema>
export type AlbumInfo = z.infer<typeof albumInfoSchema>
export type ScrollbarData = z.infer<typeof scrollbarDataSchema>
export type DisplayElement = z.infer<typeof displayElementSchema>
export type Row = z.infer<typeof rowSchema>
export type RowWithOffset = z.infer<typeof rowWithOffsetSchema>
export type Prefetch = z.infer<typeof prefetchSchema>
export type PrefetchReturn = z.infer<typeof prefetchReturnSchema>
export type SubRow = z.infer<typeof SubRowSchema>
export type Share = z.infer<typeof ShareSchema>
export type ResolvedShare = z.infer<typeof ResolvedShareSchema>
export type TokenResponse = z.infer<typeof TokenResponseSchema>

// 從 Schema 推導出新的三種核心類型 (已經經過 transform)
export type UnifiedData = z.infer<typeof BackendDataParser>
export type GalleryImage = Extract<UnifiedData, { type: 'image' }>
export type GalleryVideo = Extract<UnifiedData, { type: 'video' }>
export type GalleryAlbum = Extract<UnifiedData, { type: 'album' }>

// 帶有 thumbhashUrl 的資料類型
export type EnrichedUnifiedData = UnifiedData & { thumbhashUrl: string | null; timestamp: number }

// 供列表使用的數據結構 - 使用扁平化結構
export interface SlicedData {
  index: number
  data: EnrichedUnifiedData
  hashToken: string
}

// 其他共用型別
export type Sorting = 'ascending' | 'descending' | 'random' | 'similar'
export type IsolationId = 'mainId' | 'subId' | 'tempId' | 'shareId'
export type FetchDataMethod = 'batch' | 'single'
export type MessageColor = 'error' | 'success' | 'info'

export interface Message {
  text: string
  color: MessageColor
}

export interface EditShareData {
  albumId: string
  share: Share
  displayName: string
}
