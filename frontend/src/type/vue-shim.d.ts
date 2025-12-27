// vue-shim.d.ts
declare module '*.vue' {
  import { DefineComponent } from 'vue'
  const component: DefineComponent<unknown, unknown, unknown>
  export default component
}

declare module '*/dataWorker?worker&inline' {
  const workerConstructor: new () => Worker
  export default workerConstructor
}
