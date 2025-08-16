<template>
  <v-progress-linear
    id="progress-bar"
    class="position-absolute w-100"
    indeterminate
    height="2"
    :style="{
      zIndex: 100
    }"
    :color="initializedStore.initialized ? 'transparent' : 'primary'"
  >
  </v-progress-linear>
</template>
<script setup lang="ts">
import { IsolationId } from '@type/types'
import { useInitializedStore } from '@/store/initializedStore'

const props = defineProps<{
  isolationId: IsolationId
}>()

const initializedStore = useInitializedStore(props.isolationId)
</script>
