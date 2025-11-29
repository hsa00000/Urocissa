<template>
  <v-toolbar flat height="2" class="no-select bg-surface">
    <v-progress-linear
      id="progress-bar"
      class="bg-surface"
      v-if="!initializedStore.initialized"
      indeterminate
      color="primary"
      height="2"
      :style="{
        border: 0
      }"
    >
    </v-progress-linear>
  </v-toolbar>
</template>
<script setup lang="ts">
import { IsolationId } from '@type/types'
import { useInitializedStore } from '@/store/initializedStore'

const props = defineProps<{
  isolationId: IsolationId
}>()

const initializedStore = useInitializedStore(props.isolationId)
</script>
