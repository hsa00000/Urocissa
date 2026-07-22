import type { UploadPresetAlbum, UploadTarget } from '@/store/uploadStore'

export interface SharedUploadIngress {
  albumId: string
  shareId: string
  password?: string | null
  albumTitle?: string | null
  uploadAllowed: boolean
}

export interface UploadIngressContext {
  routeLevel: number | undefined
  isUploadPage: boolean
  routeAlbum?: UploadPresetAlbum
  sharedUpload?: SharedUploadIngress
  presignedAlbums: readonly UploadPresetAlbum[]
  presignedTags: readonly string[]
}

export type UploadIngressResolution =
  | { kind: 'blocked-route' }
  | { kind: 'blocked-share' }
  | {
      kind: 'allowed'
      target?: UploadTarget
      showSummaryPanel: boolean
    }

export function isUploadIngressAllowed(routeLevel: number | undefined): boolean {
  return routeLevel !== 2 && routeLevel !== 4
}

export function resolveUploadIngress(context: UploadIngressContext): UploadIngressResolution {
  if (!isUploadIngressAllowed(context.routeLevel)) return { kind: 'blocked-route' }

  if (context.sharedUpload !== undefined) {
    const sharedUpload = context.sharedUpload
    if (!sharedUpload.uploadAllowed) return { kind: 'blocked-share' }

    return {
      kind: 'allowed',
      target: {
        albums: [{ id: sharedUpload.albumId, name: sharedUpload.albumTitle }],
        share: {
          albumId: sharedUpload.albumId,
          shareId: sharedUpload.shareId,
          password: sharedUpload.password
        }
      },
      showSummaryPanel: !context.isUploadPage
    }
  }

  if (context.routeAlbum !== undefined) {
    return {
      kind: 'allowed',
      target: { albums: [context.routeAlbum] },
      showSummaryPanel: !context.isUploadPage
    }
  }

  if (context.isUploadPage) {
    const albumIds = new Set<string>()
    const albums = context.presignedAlbums.filter((album) => {
      if (album.id === '' || albumIds.has(album.id)) return false
      albumIds.add(album.id)
      return true
    })
    const tags = [
      ...new Set(context.presignedTags.map((tag) => tag.trim()).filter((tag) => tag !== ''))
    ]
    const target = albums.length > 0 || tags.length > 0 ? { albums, tags } : undefined

    return {
      kind: 'allowed',
      ...(target === undefined ? {} : { target }),
      showSummaryPanel: false
    }
  }

  return { kind: 'allowed', showSummaryPanel: true }
}
