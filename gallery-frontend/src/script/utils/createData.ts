import { thumbHashToDataURL } from 'thumbhash'
import { z } from 'zod'
import { AlbumParse, DataBaseParse } from '../common/schemas.ts'
import { AbstractData, Album, Database } from '../common/types.ts'

export function createDataBase(
  databaseParse: z.infer<typeof DataBaseParse>,
  timestamp: number,
  token: string
): Database {
  const database: Database = {
    ...databaseParse,
    timestamp: timestamp,
    thumbhashUrl: thumbHashToDataURL(databaseParse.thumbhash),
    filename: databaseParse.alias[0]?.file.split('/').pop() ?? '',
    token
  }
  return database
}

export function createAlbum(
  albumParse: z.infer<typeof AlbumParse>,
  timestamp: number,
  token: string
): Album {
  const album: Album = {
    ...albumParse,
    timestamp: timestamp,
    thumbhashUrl: albumParse.thumbhash ? thumbHashToDataURL(albumParse.thumbhash) : null,
    token
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
