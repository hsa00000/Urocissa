declare module '*/dataWorker?worker&inline' {
  const workerConstructor: new () => Worker
  export default workerConstructor
}
