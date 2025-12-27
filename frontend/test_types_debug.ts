
import { z } from 'zod'
import { BackendDataParser } from './src/type/schemas'
import { EnrichedUnifiedData, UnifiedData } from './src/type/types'

type TestUnified = UnifiedData
type TestEnriched = EnrichedUnifiedData

const t: TestEnriched = {} as any;
if (t.type === 'image') {
    const x = t.width;
}
