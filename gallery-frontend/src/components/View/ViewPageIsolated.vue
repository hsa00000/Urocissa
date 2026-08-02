<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useRouteResourceLoader } from '@/script/hook/useRouteResource'
import { hasCachedResource } from '@/script/utils/routeResourceCache'
import ViewPage from './ViewPage.vue'

const route = useRoute()
const subhash = computed(() =>
  typeof route.params.subhash === 'string' ? route.params.subhash : ''
)
const needsDirectResource = computed(
  () => subhash.value !== '' && !hasCachedResource('subId', subhash.value)
)

useRouteResourceLoader(subhash, 'subDetailId', {
  enabled: needsDirectResource,
  clearWhenDisabled: false
})
</script>

<template>
  <ViewPage
    collection-isolation-id="subId"
    direct-isolation-id="subDetailId"
    hash-param="subhash"
    :expected-types="['image', 'video']"
  />
</template>
