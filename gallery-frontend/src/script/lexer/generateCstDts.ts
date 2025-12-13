import { writeFileSync } from 'fs'
import { resolve, dirname } from 'path'
import { generateCstDts } from 'chevrotain'
import { MyParser } from './lexer.ts'
import { fileURLToPath } from 'url'
const parserInstance = new MyParser()
const productions = parserInstance.getGAstProductions()
const __dirname = dirname(fileURLToPath(import.meta.url))
const dtsString = generateCstDts(productions)
const dtsPath = resolve(__dirname, '../../type/MyParserCst.d.ts')
writeFileSync(dtsPath, dtsString)
