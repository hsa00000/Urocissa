import { CstParser, Lexer, createToken, type TokenType } from 'chevrotain'
import {
  AlbumExpressionCstChildren,
  AndExpressionCstChildren,
  AnyExpressionCstChildren,
  AtomicExpressionCstChildren,
  ExpressionCstChildren,
  ExtExpressionCstChildren,
  MakeExpressionCstChildren,
  ModelExpressionCstChildren,
  NotExpressionCstChildren,
  OrExpressionCstChildren,
  PathExpressionCstChildren,
  TagExpressionCstChildren,
  TypeExpressionCstChildren
} from './MyParserCst'
import { getArrayValue } from '../common/functions.ts'
const WhiteSpace = createToken({
  name: 'WhiteSpace',
  pattern: /\s+/,
  group: Lexer.SKIPPED
})

const OpenParenthesis: TokenType = createToken({ name: 'OpenParenthesis', pattern: /\(/ })
const CloseParenthesis: TokenType = createToken({ name: 'CloseParenthesis', pattern: /\)/ })
const Or: TokenType = createToken({ name: 'Or', pattern: /or/ })
const And: TokenType = createToken({ name: 'And', pattern: /and/ })
const Not: TokenType = createToken({ name: 'Not', pattern: /not/ })
const Tag: TokenType = createToken({ name: 'Tag', pattern: /tag:/ })
const Type: TokenType = createToken({ name: 'Type', pattern: /type:/ })
const Ext: TokenType = createToken({ name: 'Ext', pattern: /ext:/ })
const Model: TokenType = createToken({ name: 'Model', pattern: /model:/ })
const Make: TokenType = createToken({ name: 'Makel', pattern: /make:/ })
const Album: TokenType = createToken({ name: 'Album', pattern: /album:/ })
const Path: TokenType = createToken({ name: 'Path', pattern: /path:/ })
const Any: TokenType = createToken({ name: 'Any', pattern: /any:/ })
const Comma: TokenType = createToken({ name: 'Comma', pattern: /,/ })

const Identifier: TokenType = createToken({
  name: 'Identifier',
  pattern:
    /[\u0030-\u0039\u0041-\u005A\u0061-\u007A\u4E00-\u9FFF\u3040-\u309F\u30A0-\u30FF\uAC00-\uD7AF\u0370-\u03FF\u0400-\u04FF_\u002D\u0020]+/
  /*
  \u0030-\u0039: Decimal digits 0 to 9.
  \u0041-\u005A: Uppercase Latin letters A to Z.
  \u0061-\u007A: Lowercase Latin letters a to z.
  \u4E00-\u9FFF: Common CJK (Chinese-Japanese-Korean) ideographs, including a significant portion of Chinese characters and some Japanese and Korean characters.
  \u3040-\u309F: Hiragana characters (Japanese script).
  \u30A0-\u30FF: Katakana characters (Japanese script).
  \uAC00-\uD7AF: Hangul syllables (Korean script).
  \u0370-\u03FF: Greek and Coptic characters.
  \u0400-\u04FF: Cyrillic characters.
  _: Underscore character.
  \u002D: Hyphen character (-).
  \u0020: Space character.
*/
})

const allTokens: TokenType[] = [
  WhiteSpace,
  OpenParenthesis,
  CloseParenthesis,
  Or,
  And,
  Not,
  Tag,
  Type,
  Ext,
  Make,
  Album,
  Model,
  Path,
  Any,
  Comma,
  Identifier
]

export const MyLexer: Lexer = new Lexer(allTokens)

export class MyParser extends CstParser {
  constructor() {
    super(allTokens)
    this.performSelfAnalysis()
  }

  public expression = this.RULE('expression', () => {
    this.OR([
      { ALT: () => this.SUBRULE(this.orExpression) },
      { ALT: () => this.SUBRULE(this.andExpression) },
      { ALT: () => this.SUBRULE(this.notExpression) },
      { ALT: () => this.SUBRULE(this.atomicExpression) }
    ])
  })

  public orExpression = this.RULE('orExpression', () => {
    this.CONSUME1(Or)
    this.CONSUME2(OpenParenthesis)
    this.SUBRULE1(this.expression)
    this.MANY(() => {
      this.CONSUME3(Comma)
      this.SUBRULE2(this.expression)
    })
    this.CONSUME4(CloseParenthesis)
  })

  public andExpression = this.RULE('andExpression', () => {
    this.CONSUME1(And)
    this.CONSUME2(OpenParenthesis)
    this.SUBRULE1(this.expression)
    this.MANY(() => {
      this.CONSUME3(Comma)
      this.SUBRULE2(this.expression)
    })
    this.CONSUME4(CloseParenthesis)
  })

  public atomicExpression = this.RULE('atomicExpression', () => {
    this.OR([
      { ALT: () => this.SUBRULE(this.tagExpression) },
      { ALT: () => this.SUBRULE(this.typeExpression) },
      { ALT: () => this.SUBRULE(this.extExpression) },
      { ALT: () => this.SUBRULE(this.makeExpression) },
      { ALT: () => this.SUBRULE(this.modelExpression) },
      { ALT: () => this.SUBRULE(this.albumExpression) },
      { ALT: () => this.SUBRULE(this.pathExpression) },
      { ALT: () => this.SUBRULE(this.anyExpression) }
    ])
  })

  public notExpression = this.RULE('notExpression', () => {
    this.CONSUME1(Not)
    this.CONSUME2(OpenParenthesis)
    this.SUBRULE(this.expression)
    this.CONSUME3(CloseParenthesis)
  })

  public tagExpression = this.RULE('tagExpression', () => {
    this.CONSUME1(Tag)
    this.CONSUME2(Identifier)
  })

  public typeExpression = this.RULE('typeExpression', () => {
    this.CONSUME1(Type)
    this.CONSUME2(Identifier)
  })
  public extExpression = this.RULE('extExpression', () => {
    this.CONSUME1(Ext)
    this.CONSUME2(Identifier)
  })
  public makeExpression = this.RULE('makeExpression', () => {
    this.CONSUME1(Make)
    this.CONSUME2(Identifier)
  })
  public modelExpression = this.RULE('modelExpression', () => {
    this.CONSUME1(Model)
    this.CONSUME2(Identifier)
  })
  public albumExpression = this.RULE('albumExpression', () => {
    this.CONSUME1(Album)
    this.CONSUME2(Identifier)
  })
  public pathExpression = this.RULE('pathExpression', () => {
    this.CONSUME1(Path)
    this.CONSUME2(Identifier)
  })
  public anyExpression = this.RULE('anyExpression', () => {
    this.CONSUME1(Any)
    this.CONSUME2(Identifier)
  })
}

const parserInstance: MyParser = new MyParser()
const BaseVisitor = parserInstance.getBaseCstVisitorConstructor<unknown, unknown>()
export class MyVisitor extends BaseVisitor {
  constructor() {
    super()
    this.validateVisitor()
  }

  expression(children: ExpressionCstChildren) {
    if (children.orExpression) {
      return this.visit(children.orExpression)
    }
    if (children.andExpression) {
      return this.visit(children.andExpression)
    }
    if (children.notExpression) {
      return this.visit(children.notExpression)
    }
    if (children.atomicExpression) {
      return this.visit(children.atomicExpression)
    }
  }

  // Visit an orExpression node
  orExpression(children: OrExpressionCstChildren) {
    const expressions = children.expression.map((expr) => this.visit(expr))
    return { Or: expressions }
  }

  // Visit an andExpression node
  andExpression(children: AndExpressionCstChildren) {
    const expressions = children.expression.map((expr) => this.visit(expr))
    return { And: expressions }
  }

  // Visit a notExpression node
  notExpression(children: NotExpressionCstChildren) {
    const expression = this.visit(children.expression)
    return { Not: expression }
  }

  // Visit an atomicExpression node
  atomicExpression(children: AtomicExpressionCstChildren) {
    if (children.tagExpression) {
      return this.visit(children.tagExpression)
    }
    if (children.typeExpression) {
      return this.visit(children.typeExpression)
    }
    if (children.extExpression) {
      return this.visit(children.extExpression)
    }
    if (children.makeExpression) {
      return this.visit(children.makeExpression)
    }
    if (children.modelExpression) {
      return this.visit(children.modelExpression)
    }
    if (children.albumExpression) {
      return this.visit(children.albumExpression)
    }
    if (children.pathExpression) {
      return this.visit(children.pathExpression)
    }
    if (children.anyExpression) {
      return this.visit(children.anyExpression)
    }
  }

  // Visit a tagExpression node
  tagExpression(children: TagExpressionCstChildren) {
    return { Tag: getArrayValue(children.Identifier, 0).image }
  }

  typeExpression(children: TypeExpressionCstChildren) {
    return { ExtType: getArrayValue(children.Identifier, 0).image }
  }

  extExpression(children: ExtExpressionCstChildren) {
    return { Ext: getArrayValue(children.Identifier, 0).image }
  }
  makeExpression(children: MakeExpressionCstChildren) {
    return { Make: getArrayValue(children.Identifier, 0).image }
  }
  modelExpression(children: ModelExpressionCstChildren) {
    return { Model: getArrayValue(children.Identifier, 0).image }
  }
  albumExpression(children: AlbumExpressionCstChildren) {
    return { Album: getArrayValue(children.Identifier, 0).image }
  }
  pathExpression(children: PathExpressionCstChildren) {
    return { Path: getArrayValue(children.Identifier, 0).image }
  }
  anyExpression(children: AnyExpressionCstChildren) {
    return { Any: getArrayValue(children.Identifier, 0).image }
  }
}
