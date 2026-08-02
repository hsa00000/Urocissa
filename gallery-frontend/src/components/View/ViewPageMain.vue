<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useRouteResourceLoader } from '@/script/hook/useRouteResource'
import { hasCachedResource } from '@/script/utils/routeResourceCache'
import ViewPage from './ViewPage.vue'

const route = useRoute()
const hash = computed(() =>
  typeof route.params.hash === 'string' ? route.params.hash : ''
)
const isShareRoute = computed(() => route.meta.baseName === 'share')
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
</script>

<template>
  <ViewPage
    collection-isolation-id="mainId"
    :direct-isolation-id="isShareRoute ? undefined : 'detailId'"
    hash-param="hash"
  />
</template>
