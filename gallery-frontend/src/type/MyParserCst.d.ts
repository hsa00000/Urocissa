/* eslint-disable @typescript-eslint/consistent-type-definitions */
import type { CstNode, ICstVisitor, IToken } from 'chevrotain'

export interface ExpressionCstNode extends CstNode {
  name: 'expression'
  children: ExpressionCstChildren
}

export type ExpressionCstChildren = {
  orExpression?: OrExpressionCstNode[]
  andExpression?: AndExpressionCstNode[]
  notExpression?: NotExpressionCstNode[]
  atomicExpression?: AtomicExpressionCstNode[]
}

export interface OrExpressionCstNode extends CstNode {
  name: 'orExpression'
  children: OrExpressionCstChildren
}

export type OrExpressionCstChildren = {
  Or: IToken[]
  OpenParenthesis: IToken[]
  expression: ExpressionCstNode[]
  Comma?: IToken[]
  CloseParenthesis: IToken[]
}

export interface AndExpressionCstNode extends CstNode {
  name: 'andExpression'
  children: AndExpressionCstChildren
}

export type AndExpressionCstChildren = {
  And: IToken[]
  OpenParenthesis: IToken[]
  expression: ExpressionCstNode[]
  Comma?: IToken[]
  CloseParenthesis: IToken[]
}

export interface AtomicExpressionCstNode extends CstNode {
  name: 'atomicExpression'
  children: AtomicExpressionCstChildren
}

export type AtomicExpressionCstChildren = {
  tagExpression?: TagExpressionCstNode[]
  typeExpression?: TypeExpressionCstNode[]
  extExpression?: ExtExpressionCstNode[]
  makeExpression?: MakeExpressionCstNode[]
  modelExpression?: ModelExpressionCstNode[]
  albumExpression?: AlbumExpressionCstNode[]
  pathExpression?: PathExpressionCstNode[]
  anyExpression?: AnyExpressionCstNode[]
}

export interface NotExpressionCstNode extends CstNode {
  name: 'notExpression'
  children: NotExpressionCstChildren
}

export type NotExpressionCstChildren = {
  Not: IToken[]
  OpenParenthesis: IToken[]
  expression: ExpressionCstNode[]
  CloseParenthesis: IToken[]
}

export interface TagExpressionCstNode extends CstNode {
  name: 'tagExpression'
  children: TagExpressionCstChildren
}

export type TagExpressionCstChildren = {
  Tag: IToken[]
  Identifier: IToken[]
}

export interface TypeExpressionCstNode extends CstNode {
  name: 'typeExpression'
  children: TypeExpressionCstChildren
}

export type TypeExpressionCstChildren = {
  Type: IToken[]
  Identifier: IToken[]
}

export interface ExtExpressionCstNode extends CstNode {
  name: 'extExpression'
  children: ExtExpressionCstChildren
}

export type ExtExpressionCstChildren = {
  Ext: IToken[]
  Identifier: IToken[]
}

export interface MakeExpressionCstNode extends CstNode {
  name: 'makeExpression'
  children: MakeExpressionCstChildren
}

export type MakeExpressionCstChildren = {
  Makel: IToken[]
  Identifier: IToken[]
}

export interface ModelExpressionCstNode extends CstNode {
  name: 'modelExpression'
  children: ModelExpressionCstChildren
}

export type ModelExpressionCstChildren = {
  Model: IToken[]
  Identifier: IToken[]
}

export interface AlbumExpressionCstNode extends CstNode {
  name: 'albumExpression'
  children: AlbumExpressionCstChildren
}

export type AlbumExpressionCstChildren = {
  Album: IToken[]
  Identifier: IToken[]
}

export interface PathExpressionCstNode extends CstNode {
  name: 'pathExpression'
  children: PathExpressionCstChildren
}

export type PathExpressionCstChildren = {
  Path: IToken[]
  Identifier: IToken[]
}

export interface AnyExpressionCstNode extends CstNode {
  name: 'anyExpression'
  children: AnyExpressionCstChildren
}

export type AnyExpressionCstChildren = {
  Any: IToken[]
  Identifier: IToken[]
}

export interface ICstNodeVisitor<IN, OUT> extends ICstVisitor<IN, OUT> {
  expression(children: ExpressionCstChildren, param?: IN): OUT
  orExpression(children: OrExpressionCstChildren, param?: IN): OUT
  andExpression(children: AndExpressionCstChildren, param?: IN): OUT
  atomicExpression(children: AtomicExpressionCstChildren, param?: IN): OUT
  notExpression(children: NotExpressionCstChildren, param?: IN): OUT
  tagExpression(children: TagExpressionCstChildren, param?: IN): OUT
  typeExpression(children: TypeExpressionCstChildren, param?: IN): OUT
  extExpression(children: ExtExpressionCstChildren, param?: IN): OUT
  makeExpression(children: MakeExpressionCstChildren, param?: IN): OUT
  modelExpression(children: ModelExpressionCstChildren, param?: IN): OUT
  albumExpression(children: AlbumExpressionCstChildren, param?: IN): OUT
  pathExpression(children: PathExpressionCstChildren, param?: IN): OUT
  anyExpression(children: AnyExpressionCstChildren, param?: IN): OUT
}
