<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useRouteResourceLoader } from '@/script/hook/useRouteResource'
import ViewPage from './ViewPage.vue'

const route = useRoute()
const hash = computed(() =>
  typeof route.params.hash === 'string' ? route.params.hash : ''
)
const isShareRoute = computed(() => route.meta.baseName === 'share')

useRouteResourceLoader(hash, 'detailId', () => !isShareRoute.value)
</script>

<template>
  <ViewPage
    collection-isolation-id="mainId"
    :direct-isolation-id="isShareRoute ? undefined : 'detailId'"
    hash-param="hash"
  />
</template>
