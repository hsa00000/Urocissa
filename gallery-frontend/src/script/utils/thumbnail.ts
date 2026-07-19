export function getThumbnailSrc(hash: string, cacheVersion: number): string {
  const fileName = cacheVersion === 0 ? `${hash}.jpg` : `${hash}-v${cacheVersion}.jpg`
  return `/object/compressed/${hash.slice(0, 2)}/${fileName}`
}
