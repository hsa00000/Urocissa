import { z } from 'zod'
import { fixedBigRowHeight } from '@/type/constants'

export const AliasSchema = z.object({
  file: z.string(),
  modified: z.number(),
  scanTime: z.number()
})

export const displayElementSchema = z.object({
  displayWidth: z.number(),
  displayHeight: z.number(),
  displayTopPixelAccumulated: z.number().optional().default(0)
})

export const rowSchema = z.object({
  start: z.number(),
  end: z.number(),
  rowHeight: z.number().optional().default(fixedBigRowHeight),
  displayElements: z.array(displayElementSchema),
  topPixelAccumulated: z.number().default(0),
  rowIndex: z.number(),
  offset: z.number().optional().default(0)
})

export const rowWithOffsetSchema = z.object({
  row: rowSchema,
  offset: z.number(),
  windowWidth: z.number()
})

export const prefetchSchema = z.object({
  timestamp: z.number(),
  dataLength: z.number(),
  locateTo: z.number().nullable()
})

export const ShareSchema = z.object({
  url: z.string().max(64),
  description: z.string(),
  password: z.string().nullable(),
  showMetadata: z.boolean(),
  showDownload: z.boolean(),
  showUpload: z.boolean(),
  exp: z.number()
})

export const ResolvedShareSchema = ShareSchema.extend({
  albumId: z.string().max(64),
  albumTitle: z.string().nullable()
})

export const prefetchReturnSchema = z
  .object({
    prefetch: prefetchSchema,
    token: z.string(),
    resolvedShareOpt: ResolvedShareSchema.nullable()
  })
  .transform((data) => ({
    prefetch: data.prefetch,
    token: data.token,
    resolvedShare: data.resolvedShareOpt
  }))

export const DataBaseParse = z.object({
  album: z.array(z.string()),
  ext: z.string(),
  extType: z.string(),
  hash: z.string(),
  height: z.number(),
  pending: z.boolean(),
  phash: z.array(z.number()),
  size: z.number(),
  thumbhash: z.array(z.number()),
  width: z.number(),
  timestampMs: z.number(),
  description: z.string().nullable().optional()
})

// --- New Flat Schemas (Matching Backend Response) ---

const ObjTypeEnum = z.enum(['image', 'video', 'album'])

const FlatObjectBase = z.object({
  id: z.string(),
  objType: ObjTypeEnum,
  createdTime: z.number(),
  pending: z.boolean(),
  thumbhash: z.array(z.number()).nullable().optional(),
  description: z.string().nullable().optional(),
  tags: z.array(z.string()).default([]) // [Modified]: Tags moved to ObjectSchema in backend
})

export const DataBaseSchema = DataBaseParse.extend({
  timestamp: z.number(),
  thumbhashUrl: z.string(), // need initialize
  filename: z.string(), // need initialize
  tags: z.array(z.string()),
  exifVec: z.record(z.string(), z.string()),
  object: FlatObjectBase
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
  userDefinedMetadata: z.record(z.string(), z.array(z.string())),
  shareList: z
    .record(z.string(), ShareSchema)
    .optional()
    .default({})
    .transform((obj) => new Map(Object.entries(obj))),
  tag: z.array(z.string()),
  itemCount: z.number(),
  itemSize: z.number(),
  pending: z.boolean(),
  description: z.string().nullable().optional()
})

export const AlbumSchema = AlbumParse.extend({
  timestamp: z.number(),
  thumbhashUrl: z.string().nullable(), // need initialize
  tags: z.array(z.string()),
  object: FlatObjectBase
})

// --- New Flat Schemas (Matching Backend Response) ---

export const FlatImageSchema = FlatObjectBase.extend({
  objType: z.literal('image'),
  type: z.literal('image').optional(), // Backend's MediaCombined tag
  size: z.number(),
  width: z.number(),
  height: z.number(),
  ext: z.string(),
  phash: z.array(z.number()).nullable().optional(),
  exifVec: z.record(z.string(), z.string()).default({}), // [Modified]: Exif moved to ImageCombined
  albums: z.array(z.string()).default([]) // [Added]: Backend returns albums list
})

export const FlatVideoSchema = FlatObjectBase.extend({
  objType: z.literal('video'),
  type: z.literal('video').optional(), // Backend's MediaCombined tag
  size: z.number(),
  width: z.number(),
  height: z.number(),
  ext: z.string(),
  duration: z.number().default(0),
  exifVec: z.record(z.string(), z.string()).default({}), // [Modified]: Exif moved to VideoCombined
  albums: z.array(z.string()).default([]) // [Added]: Backend returns albums list
})

export const FlatAlbumSchema = FlatObjectBase.extend({
  objType: z.literal('album'),
  title: z.string().nullable(),
  startTime: z.number().nullable(),
  endTime: z.number().nullable(),
  lastModifiedTime: z.number(),
  cover: z.string().nullable(),
  userDefinedMetadata: z.record(z.string(), z.array(z.string())),
  // tag: z.array(z.string()).optional().default([]), // [Removed]: Now using 'tags' from FlatObjectBase, but keeping legacy mapping in mind
  itemCount: z.number(),
  itemSize: z.number()
})

// REPLACED: Now uses the flat schemas union
export const AbstractDataParseSchema = z.union([FlatImageSchema, FlatVideoSchema, FlatAlbumSchema])

export const AbstractDataWithTagSchema = z.object({
  data: AbstractDataParseSchema,
  // tag: z.array(z.string()).optional(), // [Removed]: Backend removed this
  alias: z.array(AliasSchema),
  token: z.string()
  // exifVec: z.record(z.string(), z.string()) // [Removed]: Backend removed this
})

export const AbstractDataSchema = z.object({
  database: DataBaseSchema.optional(),
  album: AlbumSchema.optional()
})

export const scrollbarDataSchema = z.object({
  index: z.number(),
  year: z.number(),
  month: z.number()
})

export const tagInfoSchema = z.object({
  tag: z.string(),
  number: z.number()
})

export const albumInfoSchema = z
  .object({
    albumId: z.string(),
    albumName: z.string().nullable(),
    shareList: z.record(z.string(), ShareSchema)
  })
  .transform((albumData) => ({
    albumId: albumData.albumId,
    albumName: albumData.albumName,
    shareList: new Map(Object.entries(albumData.shareList)),
    displayName: albumData.albumName ?? 'Untitled'
  }))

export const databaseTimestampSchema = z.object({
  abstractData: AbstractDataParseSchema,
  timestamp: z.number(),
  token: z.string()
})

export const SubRowSchema = z.object({
  displayElements: z.array(displayElementSchema)
})

export const PublicConfigSchema = z.object({
  readOnlyMode: z.boolean(),
  disableImg: z.boolean()
})

export const tokenReturnSchema = z.object({
  token: z.string()
})

export const TokenResponseSchema = z.object({
  token: z.string()
})

export const serverErrorSchema = z.object({
  error: z.string(),
  chain: z.array(z.string()).optional()
})
