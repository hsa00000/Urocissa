/**
 * Steps to regenerate @type/MyParserCst:
 * * Due to circular dependency issues (lexer.ts imports @type/MyParserCst,
 * while generateCstDts.ts imports lexer.ts), follow these steps to
 * regenerate the type definition file:
 *
 * 1. Edit src/script/lexer/lexer.ts and comment out the following imports:
 * - import { ... } from '@type/MyParserCst'
 * - import { getArrayValue } from '@utils/getter'
 * - import { unescapeAndUnwrap } from '@utils/escape'
 * This prevents module resolution errors during the generation process.
 *
 * 2. Run the generation script:
 * cd frontend
 * npm run generateLexer
 * This executes "ts-node-esm ./src/script/lexer/generateCstDts.ts",
 * which generates a new MyParserCst.d.ts file in the src/type/ directory.
 *
 * 3. Uncomment the imports in src/script/lexer/lexer.ts to restore the original state.
 *
 * 4. Check for TypeScript type errors:
 * npx vue-tsc --noEmit
 * If errors occur (e.g., 'Object is possibly undefined'), fix the
 * accessor logic (e.g., by adding null checks).
 *
 * 5. If issues persist, repeat steps 1-4 or manually verify the output
 * paths in generateCstDts.ts.
 */
import { writeFileSync } from 'fs'
import { resolve, dirname } from 'path'
import { generateCstDts } from 'chevrotain'
import { MyParser } from './lexer.ts'
import { fileURLToPath } from 'url'
const parserInstance = new MyParser()
const productions = parserInstance.getGAstProductions()
const __dirname = dirname(fileURLToPath(import.meta.url))
const dtsString = generateCstDts(productions)
const dtsPath = resolve(__dirname, 'MyParserCst.d.ts')
writeFileSync(dtsPath, dtsString)
