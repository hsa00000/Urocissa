export function serializeMetadataAsJson(metadata: object): string {
  return JSON.stringify(metadata, null, 2)
}
