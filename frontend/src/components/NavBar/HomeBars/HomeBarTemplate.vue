<template>
  <v-toolbar flat height="2" class="no-select bg-surface" />
  <slot name="content"></slot>
  <ProgressBar :isolation-id="isolationId" />
</template>
<script setup lang="ts">
import { IsolationId } from '@type/types'
import ProgressBar from '@/components/NavBar/ProgressBar.vue'

defineProps<{
  isolationId: IsolationId
}>()
</script>
