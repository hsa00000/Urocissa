<template>
  <v-list-item
    prepend-icon="mdi-open-in-new"
    value="view-original-file"
    @click="handleClick"
    target="_blank"
  >
    <v-list-item-title class="wrap">{{ 'View Original File' }}</v-list-item-title>
  </v-list-item>
</template>
<script setup lang="ts">
import { useTokenStore } from '@/store/tokenStore'

const props = defineProps<{
  src: string
  hash: string
}>()
const tokenStore = useTokenStore()

async function handleClick() {
  await tokenStore.tryRefreshAndStoreTokenToDb(props.hash)
  window.open(props.src, '_blank')
}
</script>
