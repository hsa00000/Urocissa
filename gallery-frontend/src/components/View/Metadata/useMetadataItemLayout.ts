import { computed, inject, provide } from 'vue'
import type { ComputedRef, InjectionKey } from 'vue'

export interface MetadataItemLayout {
  metadataChipSize: ComputedRef<'default' | 'small'>
  metadataItemDensity: ComputedRef<'default' | 'compact'>
  metadataItemSlim: ComputedRef<boolean>
  metadataTextClass: ComputedRef<string>
}

const MetadataItemLayoutKey: InjectionKey<MetadataItemLayout> = Symbol('MetadataItemLayout')

export function provideMetadataItemLayout(): MetadataItemLayout {
  const layout: MetadataItemLayout = {
    metadataChipSize: computed(() => 'default'),
    metadataItemDensity: computed(() => 'default'),
    metadataItemSlim: computed(() => false),
    metadataTextClass: computed(() => 'text-wrap')
  }
  provide(MetadataItemLayoutKey, layout)
  return layout
}

export function useMetadataItemLayout(): MetadataItemLayout {
  const layout = inject(MetadataItemLayoutKey)
  if (layout) return layout

  return {
    metadataChipSize: computed(() => 'default'),
    metadataItemDensity: computed(() => 'default'),
    metadataItemSlim: computed(() => false),
    metadataTextClass: computed(() => 'text-wrap')
  }
}
