import { beforeEach, describe, expect, it } from 'vitest'
import { createSSRApp, h, type FunctionalComponent } from 'vue'
import { renderToString } from '@vue/server-renderer'
import { createPinia, setActivePinia } from 'pinia'
import { useInitializedStore } from '@/store/initializedStore'
import ProgressBar from './ProgressBar.vue'

const VuetifyStub: FunctionalComponent = (_props, { attrs, slots }) =>
  h('div', attrs, slots.default?.())

async function renderProgressBar(initialized: boolean): Promise<string> {
  const pinia = createPinia()
  setActivePinia(pinia)
  useInitializedStore('mainId').initialized = initialized

  const app = createSSRApp(ProgressBar, { isolationId: 'mainId' })
  app.use(pinia)
  app.component('VToolbar', VuetifyStub)
  app.component('VProgressLinear', VuetifyStub)

  return renderToString(app)
}

describe('collection progress bar', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders while a collection snapshot is invalidated', async () => {
    expect(await renderProgressBar(false)).toContain('id="progress-bar"')
  })

  it('is absent after the current collection snapshot is initialized', async () => {
    expect(await renderProgressBar(true)).not.toContain('id="progress-bar"')
  })
})
