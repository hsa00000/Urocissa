// src/type/types.ts
import { z } from 'zod'
import {
  AliasSchema,
  facetValueInfoSchema,
  searchFacetsSchema,
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
  BackendDataParser,
  routeResourceSnapshotSchema,
  savedSearchContextSchema,
  savedSearchSchema
} from '@type/schemas'

// Basic types
export type Alias = z.infer<typeof AliasSchema>
export type FacetValueInfo = z.infer<typeof facetValueInfoSchema>
export type SearchFacets = z.infer<typeof searchFacetsSchema>
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
export type RouteResourceSnapshot = z.infer<typeof routeResourceSnapshotSchema>
export type SavedSearchContext = z.infer<typeof savedSearchContextSchema>
export type SavedSearch = z.infer<typeof savedSearchSchema>

// Core unified types (transformed)
export type UnifiedData = z.infer<typeof BackendDataParser>
export type GalleryImage = Extract<UnifiedData, { type: 'image' }>
export type GalleryVideo = Extract<UnifiedData, { type: 'video' }>
export type GalleryAlbum = Extract<UnifiedData, { type: 'album' }>

export type EnrichedUnifiedData = UnifiedData & { thumbhashUrl: string | null; timestamp: number }

// List view data structure
export interface SlicedData {
  index: number
  data: EnrichedUnifiedData
  hashToken: string
  hashTokenExpiresAt: number | undefined
}

export type GallerySortOrder = 'descending' | 'ascending' | 'random'
export type CollectionIsolationId = 'mainId' | 'subId' | 'tempId' | 'shareId'
export type RouteResourceIsolationId = 'detailId' | 'subDetailId'
export type IsolationId = CollectionIsolationId | RouteResourceIsolationId
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

export interface ShareFormData {
  description: string
  passwordRequired: boolean
  password: string
  expireEnabled: boolean
  expDuration: number | null
  showUpload: boolean
  showDownload: boolean
  showMetadata: boolean
}

