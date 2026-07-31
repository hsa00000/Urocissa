import { readAndCompressImage } from '@misskey-dev/browser-image-resizer'

export interface ThumbnailSize {
  width: number
  height: number
}

export function calculateThumbnailSize(
  sourceWidth: number,
  sourceHeight: number,
  maxWidth: number,
  maxHeight: number
): ThumbnailSize {
  if (sourceWidth <= 0 || sourceHeight <= 0 || maxWidth <= 0 || maxHeight <= 0) {
    throw new Error('thumbnail dimensions must be positive')
  }

  const aspectRatio = sourceWidth / sourceHeight
  const outputWidth = Math.min(sourceWidth, maxWidth, aspectRatio * maxHeight)
  const width = Math.max(1, Math.floor(outputWidth))
  const height = Math.max(
    1,
    Math.floor(Math.min(sourceHeight * (width / sourceWidth), maxHeight))
  )
  return { width, height }
}

export async function resizeThumbnailBlob(
  blob: Blob,
  displayWidth: number,
  displayHeight: number,
  devicePixelRatio: number,
  albumMode: boolean
): Promise<Blob> {
  const image = await createImageBitmap(blob)
  try {
    const coverScale = albumMode
      ? Math.max(displayWidth / image.width, displayHeight / image.height)
      : 1
    const maxWidth = albumMode
      ? image.width * coverScale * devicePixelRatio
      : displayWidth * devicePixelRatio
    const maxHeight = albumMode
      ? image.height * coverScale * devicePixelRatio
      : displayHeight * devicePixelRatio
    const target = calculateThumbnailSize(image.width, image.height, maxWidth, maxHeight)

    // Backend thumbnails are already JPEG. If no resize is needed, retaining
    // the original blob avoids a visually redundant decode/encode cycle.
    if (
      target.width === image.width &&
      target.height === image.height &&
      blob.type === 'image/jpeg'
    ) {
      return blob
    }

    // Feed the decoded pixels to the existing resizer as a canvas. This keeps
    // the established bilinear output while avoiding its second Bitmap decode.
    const canvas = new OffscreenCanvas(image.width, image.height)
    const context = canvas.getContext('2d')
    if (context === null) {
      throw new Error('thumbnail resize canvas context is unavailable')
    }

    context.drawImage(image, 0, 0)
    return await readAndCompressImage(canvas, {
      argorithm: 'bilinear',
      quality: 1,
      maxWidth,
      maxHeight
    })
  } finally {
    image.close()
  }
}
