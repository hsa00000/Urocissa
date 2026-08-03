<script setup lang="ts">
import { computed } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'
import ViewPageDisplay from '@/components/View/Display/Display.vue'
import ViewPageMetadata from '@/components/View/Metadata/ViewPageMetadata.vue'
import { useResolvedRouteResource } from '@/script/hook/useRouteResource'
import { useConstStore } from '@/store/constStore'
import { useConfigStore } from '@/store/configStore'
import { useCollectionReloadStore } from '@/store/collectionReloadStore'
import type {
  IsolationId,
  RouteResourceIsolationId
} from '@type/types'
import type { RouteResourceExpectedType } from '@/script/hook/useRouteResource'
import { interceptMobileInfoBackNavigation } from './mobileInfoBackNavigation'

const props = defineProps<{
  collectionIsolationId: IsolationId
  directIsolationId?: RouteResourceIsolationId
  hashParam: 'hash' | 'subhash'
  expectedTypes?: readonly RouteResourceExpectedType[]
  collectionOnly?: boolean
}>()

const route = useRoute()
const router = useRouter()
const constStore = useConstStore('mainId')
const configStore = useConfigStore('mainId')
const collectionReloadStore = useCollectionReloadStore('mainId')

const hash = computed(() => {
  const value = route.params[props.hashParam]
  return typeof value === 'string' ? value : ''
})

function retryCollection(): void {
  if (props.collectionIsolationId === 'subId') {
    collectionReloadStore.requestSubCollectionReload()
  } else {
    collectionReloadStore.requestMainCollectionReload()
  }
}

const { resource, status, errorMessage, retry } = useResolvedRouteResource(
  hash,
  props.collectionIsolationId,
  props.directIsolationId,
  props.expectedTypes,
  {
    collectionOnly: props.collectionOnly,
    onCollectionRetry: retryCollection
  }
)

const overlayVisible = computed<boolean>({
  get: () => true,
  set: (value) => {
    if (!value) router.back()
  }
})

const errorTitle = computed(() => {
  switch (status.value) {
    case 'invalid':
      return 'Invalid item ID'
    case 'not-found':
      return 'Item not found'
    case 'unavailable':
      return 'Item unavailable'
    case 'wrong-type':
      return 'Item cannot be opened here'
    default:
      return 'Unable to load item'
  }
})

const errorBody = computed(() => {
  if (status.value === 'wrong-type') {
    return 'This route only accepts an image or video.'
  }
  return errorMessage.value ?? 'The item could not be loaded. Please try again.'
})

function leave(): void {
  router.back()
}

function retryLoad(): void {
  void retry()
}

onBeforeRouteLeave(() =>
  interceptMobileInfoBackNavigation({
    isMobile: configStore.isMobile,
    isInfoOpen: constStore.showInfo,
    closeInfo: () => constStore.updateShowInfo(false),
    onCloseError: (error) => {
      console.error('Failed to close the mobile info panel:', error)
    }
  })
)
</script>

<template>
  <v-overlay
    id="view-page"
    v-model="overlayVisible"
    height="100%"
    width="100%"
    content-class="w-100 h-100"
    class="d-flex"
    :transition="false"
    :close-on-back="false"
  >
    <!-- Keep the child route host alive for every parent loading/error state. -->
    <router-view />

    <v-sheet
      v-if="status === 'ready' && resource !== undefined"
      :key="`${resource.isolationId}:${hash}`"
      color="background"
      class="pa-0 h-100 w-100 d-flex position-relative"
    >
      <ViewPageDisplay
        :abstract-data="resource.data"
        :index="resource.index"
        :hash="hash"
        :isolation-id="resource.isolationId"
      />
      <ViewPageMetadata
        v-if="constStore.showInfo"
        :abstract-data="resource.data"
        :index="resource.index"
        :hash="hash"
        :isolation-id="resource.isolationId"
      />
    </v-sheet>

    <v-sheet
      v-else-if="status === 'loading' || status === 'idle'"
      color="background"
      class="h-100 w-100 d-flex flex-column align-center justify-center ga-4"
    >
      <v-progress-circular indeterminate color="primary" size="64" />
      <span class="text-body-1">Loading item…</span>
    </v-sheet>

    <v-sheet
      v-else
      color="background"
      class="h-100 w-100 d-flex align-center justify-center pa-4"
    >
      <v-alert
        type="error"
        variant="tonal"
        icon="mdi-alert-circle-outline"
        :title="errorTitle"
        max-width="640"
        class="w-100"
      >
        <p class="mb-2">{{ errorBody }}</p>
        <p class="text-body-2 text-break mb-4">Requested ID: {{ hash }}</p>
        <v-btn color="primary" variant="flat" class="me-2" @click="retryLoad">
          Retry
        </v-btn>
        <v-btn variant="text" @click="leave">Leave</v-btn>
      </v-alert>
    </v-sheet>
  </v-overlay>
</template>

<style scoped>
.v-container::-webkit-scrollbar {
  display: none;
}
</style>
