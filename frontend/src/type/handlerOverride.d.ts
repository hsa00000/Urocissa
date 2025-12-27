/* eslint-disable @typescript-eslint/no-explicit-any */

// Import the original module to augment its types
import 'typesafe-agent-events'

/**
 * Module augmentation for 'typesafe-agent-events'.
 * This augmentation modifies the 'createHandler' function to allow handlers
 * to return either `void` or `Promise<void>`. This is useful for supporting
 * both synchronous and asynchronous action handlers.
 */
declare module 'typesafe-agent-events' {
  export function createHandler<
    T extends Record<string, (payload?: any) => { type: string; payload?: any }>
  >(defs: {
    [k in keyof T]: ReturnType<T[k]> extends { type: string; payload: infer P }
      ? // The handler can now return `void` or `Promise<void>` when there is a payload
        (payload: P) => void | Promise<void>
      : // The handler can now return `void` or `Promise<void>` when there is no payload
        () => void | Promise<void>
  }): (action: { [K in keyof T]: ReturnType<T[K]> }[keyof T]) => boolean // fix me: this may need to use Promise<boolean>
}
