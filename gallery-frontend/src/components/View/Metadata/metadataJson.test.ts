import { describe, expect, it } from 'vitest'
import { serializeMetadataAsJson } from './metadataJson'

describe('metadata JSON serialization', () => {
  it('preserves raw EXIF punctuation and case exactly', () => {
    const metadata = {
      type: 'image',
      id: 'photo-id',
      exif: {
        Make: 'OPPO", "", "", ""',
        Model: 'CPH1234',
        Software: 'Camera 1.0'
      }
    }

    const result = serializeMetadataAsJson(metadata)

    expect(JSON.parse(result)).toEqual(metadata)
  })

  it('formats copied metadata as readable indented JSON', () => {
    expect(serializeMetadataAsJson({ exif: { Make: 'OPPO' } })).toBe(
      '{\n  "exif": {\n    "Make": "OPPO"\n  }\n}'
    )
  })
})
