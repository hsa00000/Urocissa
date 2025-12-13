/**
 * 生成 @type/MyParserCst 的詳細過程：
 *
 * 由於循環依賴問題（lexer.ts 導入了 @type/MyParserCst，而 generateCstDts.ts 又導入了 lexer.ts），
 * 需要按以下步驟操作來重新生成類型定義文件：
 *
 * 1. 編輯 src/script/lexer/lexer.ts，註釋掉以下導入：
 *    - import { ... } from '@type/MyParserCst'
 *    - import { getArrayValue } from '@utils/getter'
 *    - import { unescapeAndUnwrap } from '@utils/escape'
 *    這是為了避免模塊解析錯誤。
 *
 * 2. 運行生成腳本：
 *    cd gallery-frontend
 *    npm run generateLexer
 *    此命令會執行 ts-node-esm ./src/script/lexer/generateCstDts.ts，生成新的 MyParserCst.d.ts 文件到 src/type/ 目錄。
 *
 * 3. 取消註釋 src/script/lexer/lexer.ts 中的導入（恢復原始狀態）。
 *
 * 4. 檢查 TypeScript 類型錯誤：
 *    npx vue-tsc --noEmit
 *    如果有錯誤（如 Object is possibly 'undefined'），需要修復訪問器中的代碼，例如添加 null 檢查。
 *
 * 5. 如果仍有問題，重複步驟 1-4，或手動調整 generateCstDts.ts 中的路徑確保寫入正確位置。
 *
 * 注意：generateCstDts.ts 已修復路徑為 resolve(__dirname, "../../type/MyParserCst.d.ts")，確保文件生成到正確位置。
 */

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
