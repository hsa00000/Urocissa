<template>
  <v-overlay
    :model-value="true"
    :height="'100%'"
    :width="'100%'"
    class="d-flex"
    id="view-page"
    transition="false"
    :close-on-back="false"
    persistent
  >
    <v-container
      v-if="index !== undefined"
      fluid
      class="pa-0 h-100 overflow-hidden position-relative"
      :style="{ backgroundColor: 'black' }"
    >
      <v-row no-gutters class="w-100 h-100 flex-nowrap">
        <ViewPageDisplay
          :abstract-data="abstractData"
          :index="index"
          :hash="hash"
          :isolation-id="isolationId"
        />
        <MetadataCol
          v-if="abstractData"
          :abstract-data="abstractData"
          :index="index"
          :hash="hash"
          :isolation-id="isolationId"
        />
      </v-row>
    </v-container>
    <v-container
      v-else
      fluid
      class="pa-0 h-100 overflow-hidden position-relative"
      :style="{ backgroundColor: 'black' }"
    >
      <v-row class="fill-height" align="center" justify="center">
        <v-progress-circular indeterminate color="primary" size="64" />
      </v-row>
    </v-container>
  </v-overlay>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useDataStore } from '@/store/dataStore'
import ViewPageDisplay from '@/components/View/Display/Display.vue'
import MetadataCol from '@/components/View/Metadata/ViewPageMetadata.vue'
import { IsolationId } from '@type/types'

const props = defineProps<{
  isolationId: IsolationId
}>()

const dataStore = useDataStore(props.isolationId)
const route = useRoute()

const hash = computed(() => {
  if (props.isolationId === 'mainId') {
    return route.params.hash as string
  } else {
    return route.params.subhash as string
  }
})

const index = computed(() => {
  return dataStore.hashMapData.get(hash.value)
})

const abstractData = computed(() => {
  if (index.value !== undefined) {
    return dataStore.data.get(index.value)
  } else {
    return undefined
  }
})
</script>
<style scoped>
.v-container::-webkit-scrollbar {
  display: none;
  /* Hide scrollbar */
}
</style>
