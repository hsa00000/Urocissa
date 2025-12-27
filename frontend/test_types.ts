
import { z } from 'zod'
import { BackendDataParser } from './src/type/schemas'
import { UnifiedData, EnrichedUnifiedData } from './src/type/types'

type TestUnified = z.infer<typeof BackendDataParser>
type TestEnriched = EnrichedUnifiedData

const t1: TestUnified = { type: 'image' } as any
const t2: TestUnified = { type: 'album' } as any

const e1: TestEnriched = { type: 'image' } as any
const e2: TestEnriched = { type: 'album' } as any

function check(data: EnrichedUnifiedData) {
    if (data.type === 'image') {}
    else if (data.type === 'video') {}
    else if (data.type === 'album') {}
}
