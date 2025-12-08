import { thumbHashToDataURL } from 'thumbhash'
import { z } from 'zod'
import { AlbumParse, DataBaseParse, AliasSchema } from '@type/schemas'
import { AbstractData, Album, Database } from '@type/types'

export function createDataBase(
  databaseParse: z.infer<typeof DataBaseParse>,
  timestamp: number,
  tags: string[] = [],
  alias: z.infer<typeof AliasSchema>[],
  exifVec: Record<string, string>
): Database {
  const objType = databaseParse.ext === 'mp4' ? 'video' : 'image'
  const database: Database = {
    ...databaseParse,
    timestamp: timestamp,
    thumbhashUrl: thumbHashToDataURL(databaseParse.thumbhash),
    filename: alias[0]?.file.split('/').pop() ?? '',
    tags: tags,
    exifVec: exifVec,
    object: {
      id: databaseParse.hash,
      objType: objType as 'image' | 'video',
      createdTime: databaseParse.timestampMs,
      pending: databaseParse.pending,
      thumbhash: databaseParse.thumbhash,
      description: databaseParse.description ?? null,
      tags: tags
    }
  }
  return database
}

export function createAlbum(
  albumParse: z.infer<typeof AlbumParse>,
  timestamp: number,
  tags: string[] = []
): Album {
  const album: Album = {
    ...albumParse,
    timestamp: timestamp,
    thumbhashUrl: albumParse.thumbhash ? thumbHashToDataURL(albumParse.thumbhash) : null,
    tags: tags,
    object: {
      id: albumParse.id,
      objType: 'album',
      createdTime: albumParse.createdTime,
      pending: albumParse.pending,
      thumbhash: albumParse.thumbhash,
      description: albumParse.description ?? null,
      tags: tags
    }
  }
  return album
}

export function createAbstractData(data: Database | Album): AbstractData {
  if ('hash' in data) {
    return { database: data }
  } else {
    return { album: data }
  }
}
