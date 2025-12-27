import { thumbHashToDataURL } from 'thumbhash'
import { UnifiedData } from '@type/types'

/**
 * 為 UnifiedData 添加 thumbhashUrl 屬性
 * 後端回傳的資料經過 Zod transform 後已經是扁平結構
 */
export function enrichWithThumbhash<T extends UnifiedData>(
  data: T
): T & { thumbhashUrl: string | null } {
  const thumbhashUrl = data.thumbhash ? thumbHashToDataURL(data.thumbhash) : null
  return { ...data, thumbhashUrl }
}

/**
 * 取得顯示用的 filename
 */
export function getFilename(data: UnifiedData): string {
  if (data.type === 'image' || data.type === 'video') {
    return data.alias[0]?.file.split('/').pop() ?? ''
  }
  return data.title ?? ''
}
