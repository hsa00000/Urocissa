import { escapeAndWrap } from '@utils/escape'

export type AdvancedSearchMediaType = 'all' | 'image' | 'video' | 'album'

export interface AdvancedSearchCriteria {
  keyword: string
  filename: string
  tag: string | null
  extension: string
  cameraMake: string
  cameraModel: string
  mediaType: AdvancedSearchMediaType
}

export function createEmptyAdvancedSearchCriteria(): AdvancedSearchCriteria {
  return {
    keyword: '',
    filename: '',
    tag: null,
    extension: '',
    cameraMake: '',
    cameraModel: '',
    mediaType: 'all'
  }
}

export function buildAdvancedSearchFilter(criteria: AdvancedSearchCriteria): string {
  const parts: string[] = []

  const keyword = criteria.keyword.trim()
  if (keyword !== '') parts.push(`any:${escapeAndWrap(keyword)}`)

  const filename = criteria.filename.trim()
  if (filename !== '') parts.push(`path:${escapeAndWrap(filename)}`)

  const tag = criteria.tag?.trim() ?? ''
  if (tag !== '') parts.push(`tag:${escapeAndWrap(tag)}`)

  const extension = criteria.extension.trim()
  if (extension !== '') parts.push(`ext:${escapeAndWrap(extension)}`)

  const cameraMake = criteria.cameraMake.trim()
  if (cameraMake !== '') parts.push(`make:${escapeAndWrap(cameraMake)}`)

  const cameraModel = criteria.cameraModel.trim()
  if (cameraModel !== '') parts.push(`model:${escapeAndWrap(cameraModel)}`)

  if (criteria.mediaType !== 'all') {
    parts.push(`type:${escapeAndWrap(criteria.mediaType)}`)
  }

  if (parts.length === 0) return ''
  if (parts.length === 1) return parts[0] ?? ''
  return `and(${parts.join(', ')})`
}
