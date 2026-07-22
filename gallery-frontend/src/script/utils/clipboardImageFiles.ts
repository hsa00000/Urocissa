import { SUPPORTED_MEDIA_EXTENSIONS } from '@/store/uploadStore'

const supportedExtensionSet = new Set<string>(SUPPORTED_MEDIA_EXTENSIONS)

const extensionByMimeType: Readonly<Record<string, string>> = {
  'image/bmp': 'bmp',
  'image/gif': 'gif',
  'image/jpeg': 'jpg',
  'image/png': 'png',
  'image/tiff': 'tiff',
  'image/webp': 'webp',
  'image/x-ms-bmp': 'bmp'
}

interface ClipboardImageCandidate {
  file: File
  mimeType: string
}

export type ClipboardPasteEvent = Pick<
  ClipboardEvent,
  'clipboardData' | 'preventDefault' | 'target'
>

function getFileExtension(fileName: string): string {
  const separatorIndex = fileName.lastIndexOf('.')
  if (separatorIndex < 0 || separatorIndex === fileName.length - 1) return ''
  return fileName.slice(separatorIndex + 1).toLowerCase()
}

function toClipboardImageCandidate(
  file: File | null,
  itemMimeType: string
): ClipboardImageCandidate | undefined {
  if (file === null) return undefined

  const normalizedItemMimeType = itemMimeType.toLowerCase()
  const normalizedFileMimeType = file.type.toLowerCase()
  const mimeType = normalizedItemMimeType.startsWith('image/')
    ? normalizedItemMimeType
    : normalizedFileMimeType.startsWith('image/')
      ? normalizedFileMimeType
      : undefined

  return mimeType === undefined ? undefined : { file, mimeType }
}

function normalizeClipboardImageFile(
  candidate: ClipboardImageCandidate,
  index: number,
  timestamp: number
): File {
  const { file, mimeType } = candidate
  const currentExtension = getFileExtension(file.name)
  const hasSupportedExtension = supportedExtensionSet.has(currentExtension)
  const inferredExtension = extensionByMimeType[mimeType]

  if (!hasSupportedExtension && inferredExtension === undefined) return file

  const name = hasSupportedExtension
    ? file.name
    : `clipboard-${timestamp}-${index + 1}.${inferredExtension}`
  const type = file.type === '' ? mimeType : file.type
  const lastModified = file.lastModified > 0 ? file.lastModified : timestamp

  if (name === file.name && type === file.type && lastModified === file.lastModified) {
    return file
  }

  return new File([file], name, { type, lastModified })
}

/**
 * Extracts image files from a paste event without requesting asynchronous clipboard permission.
 * DataTransfer.files is only used when items produced no images, preventing duplicate entries.
 */
export function extractClipboardImageFiles(
  clipboardData: DataTransfer,
  timestamp = Date.now()
): File[] {
  const itemCandidates = Array.from(clipboardData.items).flatMap((item) => {
    if (item.kind !== 'file') return []
    const candidate = toClipboardImageCandidate(item.getAsFile(), item.type)
    return candidate === undefined ? [] : [candidate]
  })

  const candidates =
    itemCandidates.length > 0
      ? itemCandidates
      : Array.from(clipboardData.files).flatMap((file) => {
          const candidate = toClipboardImageCandidate(file, file.type)
          return candidate === undefined ? [] : [candidate]
        })

  return candidates.map((candidate, index) =>
    normalizeClipboardImageFile(candidate, index, timestamp)
  )
}

/**
 * Gives clipboard images precedence over native paste content, regardless of the focused element.
 */
export function handleClipboardImagePaste(
  event: ClipboardPasteEvent,
  enqueueFiles: (files: readonly File[]) => void,
  timestamp = Date.now()
): boolean {
  if (event.clipboardData === null) return false

  const files = extractClipboardImageFiles(event.clipboardData, timestamp)
  if (files.length === 0) return false

  event.preventDefault()
  enqueueFiles(files)
  return true
}
