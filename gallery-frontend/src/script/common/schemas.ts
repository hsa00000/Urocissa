// schemas.ts

import { z } from 'zod'
import { fixedBigRowHeight } from './constants'

/**
 * Schema for alias objects.
 */
export const AliasSchema = z.object({
  file: z.string(),
  modified: z.number(),
  scanTime: z.number()
})

/**
 * Schema for display elements.
 */
export const displayElementSchema = z.object({
  displayWidth: z.number(),
  displayHeight: z.number(),
  displayTopPixelAccumulated: z.number().optional().default(0)
})

/**
 * Schema for rows.
 */
export const rowSchema = z.object({
  start: z.number(),
  end: z.number(),
  rowHeight: z.number().optional().default(fixedBigRowHeight),
  displayElements: z.array(displayElementSchema),
  topPixelAccumulated: z.number().default(0), // Changed line
  rowIndex: z.number(),
  offset: z.number().optional().default(0)
})
/**
 * Schema for rows with offset.
 */
export const rowWithOffsetSchema = z.object({
  row: rowSchema,
  offset: z.number(),
  windowWidth: z.number()
})

/**
 * Schema for prefetching data.
 */
export const prefetchSchema = z.object({
  timestamp: z.number(),
  dataLength: z.number(),
  locateTo: z.number().nullable(),
  token: z.string()
})

export const DataBaseParse = z.object({
  album: z.array(z.string()),
  alias: z.array(AliasSchema),
  exif_vec: z.record(z.string(), z.string()),
  ext: z.string(),
  ext_type: z.string(),
  hash: z.string(),
  height: z.number(),
  pending: z.boolean(),
  phash: z.array(z.number()),
  size: z.number(),
  tag: z.array(z.string()),
  thumbhash: z.array(z.number()),
  width: z.number()
})

/**
 * Schema for Database with additional fields.
 */
export const DataBaseSchema = DataBaseParse.extend({
  timestamp: z.number(),
  thumbhashUrl: z.string(), // need initialize
  filename: z.string(), // need initialize
  token: z.string()
})

/**
 * Schema for share information.
 */
export const ShareSchema = z.object({
  url: z.string().max(64),
  description: z.string(),
  password: z.string().nullable(),
  showMetadata: z.boolean(),
  showDownload: z.boolean(),
  showUpload: z.boolean(),
  exp: z.number()
})

export const AlbumParse = z.object({
  id: z.string(),
  title: z.string().nullable(),
  createdTime: z.number(),
  startTime: z.number().nullable(),
  endTime: z.number().nullable(),
  lastModifiedTime: z.number(),
  cover: z.string().nullable(),
  thumbhash: z.array(z.number()).nullable(),
  userDefinedMetadata: z.record(z.array(z.string())),
  shareList: z.array(ShareSchema),
  tag: z.array(z.string()),
  width: z.number(),
  height: z.number(),
  itemCount: z.number(),
  itemSize: z.number(),
  pending: z.boolean()
})

/**
 * Schema for Database with additional fields.
 */
export const AlbumSchema = AlbumParse.extend({
  timestamp: z.number(),
  thumbhashUrl: z.string().nullable(), // need initialize
  token: z.string()
})

export const AbstractDataParseSchema = z.union([
  z.object({ Database: DataBaseParse }),
  z.object({ Album: AlbumParse })
])

export const AbstractDataSchema = z.object({
  database: DataBaseSchema.optional(),
  album: AlbumSchema.optional()
})

/**
 * Schema for scrollbar data.
 */
export const scrollbarDataSchema = z.object({
  index: z.number(),
  year: z.number(),
  month: z.number()
})

/**
 * Schema for tag information.
 */
export const tagInfoSchema = z.object({
  tag: z.string(),
  number: z.number()
})

/**
 * Schema for album information.
 */
export const albumInfoSchema = z.object({
  albumId: z.string(),
  albumName: z.string().nullable(),
  displayName: z.string().optional()
})

/**
 * Schema for database timestamp.
 */
export const databaseTimestampSchema = z.object({
  abstractData: AbstractDataParseSchema,
  timestamp: z.number(),
  token: z.string()
})

/**
 * Represents a sub-row containing display elements.
 */
export const SubRowSchema = z.object({
  displayElements: z.array(displayElementSchema)
})

export const PublicConfigSchema = z.object({
  readOnlyMode: z.boolean(),
  disableImg: z.boolean()
})
