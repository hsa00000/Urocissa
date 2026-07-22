import { describe, expect, it } from 'vitest'
import {
  isUploadIngressAllowed,
  resolveUploadIngress,
  type UploadIngressContext
} from './uploadIngress'

function context(overrides: Partial<UploadIngressContext> = {}): UploadIngressContext {
  return {
    routeLevel: 1,
    isUploadPage: false,
    presignedAlbums: [],
    presignedTags: [],
    ...overrides
  }
}

describe('upload ingress resolution', () => {
  it('blocks the same detail route levels as drag and drop', () => {
    expect(isUploadIngressAllowed(1)).toBe(true)
    expect(isUploadIngressAllowed(3)).toBe(true)
    expect(resolveUploadIngress(context({ routeLevel: 2 }))).toEqual({
      kind: 'blocked-route'
    })
    expect(resolveUploadIngress(context({ routeLevel: 4 }))).toEqual({
      kind: 'blocked-route'
    })
  })

  it('applies deduplicated album and tag presets on the upload page', () => {
    expect(
      resolveUploadIngress(
        context({
          isUploadPage: true,
          presignedAlbums: [
            { id: 'album-a', name: 'A' },
            { id: 'album-a', name: 'Duplicate' },
            { id: 'album-b', name: 'B' },
            { id: '', name: 'Empty' }
          ],
          presignedTags: [' first ', 'second', 'first', '']
        })
      )
    ).toEqual({
      kind: 'allowed',
      target: {
        albums: [
          { id: 'album-a', name: 'A' },
          { id: 'album-b', name: 'B' }
        ],
        tags: ['first', 'second']
      },
      showSummaryPanel: false
    })
  })

  it('targets the current album and shows the summary away from the upload page', () => {
    expect(
      resolveUploadIngress(
        context({ routeLevel: 3, routeAlbum: { id: 'current-album', name: 'Current' } })
      )
    ).toEqual({
      kind: 'allowed',
      target: { albums: [{ id: 'current-album', name: 'Current' }] },
      showSummaryPanel: true
    })
  })

  it('uses allowed share credentials ahead of route and preset destinations', () => {
    expect(
      resolveUploadIngress(
        context({
          routeLevel: 3,
          routeAlbum: { id: 'route-album' },
          presignedAlbums: [{ id: 'preset-album' }],
          presignedTags: ['preset'],
          sharedUpload: {
            albumId: 'shared-album',
            shareId: 'share-id',
            password: 'secret',
            albumTitle: 'Shared',
            uploadAllowed: true
          }
        })
      )
    ).toEqual({
      kind: 'allowed',
      target: {
        albums: [{ id: 'shared-album', name: 'Shared' }],
        share: {
          albumId: 'shared-album',
          shareId: 'share-id',
          password: 'secret'
        }
      },
      showSummaryPanel: true
    })
  })

  it('blocks shares without upload permission', () => {
    expect(
      resolveUploadIngress(
        context({
          sharedUpload: {
            albumId: 'shared-album',
            shareId: 'share-id',
            uploadAllowed: false
          }
        })
      )
    ).toEqual({ kind: 'blocked-share' })
  })

  it('allows an unscoped upload without inventing a target', () => {
    expect(resolveUploadIngress(context())).toEqual({
      kind: 'allowed',
      showSummaryPanel: true
    })
    expect(resolveUploadIngress(context({ isUploadPage: true }))).toEqual({
      kind: 'allowed',
      showSummaryPanel: false
    })
  })
})
