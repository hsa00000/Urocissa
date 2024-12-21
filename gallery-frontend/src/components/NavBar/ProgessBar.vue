<template>
  <v-toolbar
    flat
    height="2"
    class="no-select"
    :style="{
      backgroundColor: '#212121'
    }"
  >
    <v-progress-linear
      v-if="!initializedStore.initialized"
      indeterminate
      color="primary"
      height="2"
      :style="{
        border: 0,
        backgroundColor: '#212121'
      }"
    >
    </v-progress-linear>
  </v-toolbar>
</template>
<script setup lang="ts">
import { IsolationId } from '@/script/common/types'
import { useInitializedStore } from '@/store/initializedStore'

const props = defineProps<{
  isolationId: IsolationId
}>()

const initializedStore = useInitializedStore(props.isolationId)
</script>
