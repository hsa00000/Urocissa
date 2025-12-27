import { MyLexer, MyParser, MyVisitor } from '@/script/lexer/lexer'
export function generateJsonString(inputText: string): string {
  const lexingResult = MyLexer.tokenize(inputText)
  if (lexingResult.errors.length) {
    console.warn(lexingResult.errors)
    throw new Error('Lexing errors detected')
  }

  const parser = new MyParser()
  parser.input = lexingResult.tokens
  const cst = parser.expression()
  if (parser.errors.length) {
    console.warn('Parsing errors detected')
    console.warn(parser.errors)
    throw new Error('Parsing errors detected')
  }

  const visitor = new MyVisitor()
  const json = visitor.visit(cst)

  return JSON.stringify(json)
}
