<script setup lang="ts">
import { computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useRouteResourceLoader } from '@/script/hook/useRouteResource'
import { hasCachedResource } from '@/script/utils/routeResourceCache'
import { useCollectionReloadStore } from '@/store/collectionReloadStore'
import { useInitializedStore } from '@/store/initializedStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import ViewPage from './ViewPage.vue'

const route = useRoute()
const hash = computed(() =>
  typeof route.params.hash === 'string' ? route.params.hash : ''
)
const isShareRoute = computed(() => route.meta.baseName === 'share')
const initializedStore = useInitializedStore('mainId')
const prefetchStore = usePrefetchStore('mainId')
const collectionReloadStore = useCollectionReloadStore('mainId')
const needsDirectResource = computed(
  () =>
    !isShareRoute.value &&
    hash.value !== '' &&
    !hasCachedResource('mainId', hash.value)
)

useRouteResourceLoader(hash, 'detailId', {
  enabled: needsDirectResource,
  // Once the collection acquires the same row, keep the direct snapshot as a
  // warm fallback until the whole Level 2 route chain is left.
  clearWhenDisabled: false
})

const needsShareCollectionLocate = computed(
  () =>
    isShareRoute.value &&
    hash.value !== '' &&
    initializedStore.initialized &&
    !hasCachedResource('mainId', hash.value) &&
    prefetchStore.locateResolution?.requestedId !== hash.value
)

watch(
  needsShareCollectionLocate,
  (needsLocate) => {
    if (needsLocate) collectionReloadStore.requestMainCollectionReload()
  },
  { immediate: true }
)
</script>

<template>
  <ViewPage
    collection-isolation-id="mainId"
    :direct-isolation-id="isShareRoute ? undefined : 'detailId'"
    :collection-only="isShareRoute"
    hash-param="hash"
  />
</template>
