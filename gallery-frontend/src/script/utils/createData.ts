import { thumbHashToDataURL } from 'thumbhash'
import { z } from 'zod'
import { AlbumParse, DataBaseParse } from '@type/schemas'
import { AbstractData, Album, Database } from '@type/types'

export function createDataBase(
  databaseParse: z.infer<typeof DataBaseParse>,
  timestamp: number,
  tags: string[] = []
): Database {
  const database: Database = {
    ...databaseParse,
    timestamp: timestamp,
    thumbhashUrl: thumbHashToDataURL(databaseParse.thumbhash),
    filename: databaseParse.alias[0]?.file.split('/').pop() ?? '',
    tags: tags
  }
  return database
}

export function createAlbum(albumParse: z.infer<typeof AlbumParse>, timestamp: number, tags: string[] = []): Album {
  const album: Album = {
    ...albumParse,
    timestamp: timestamp,
    thumbhashUrl: albumParse.thumbhash ? thumbHashToDataURL(albumParse.thumbhash) : null,
    tags: tags
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
