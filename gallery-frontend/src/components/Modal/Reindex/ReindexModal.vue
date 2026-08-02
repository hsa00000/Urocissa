<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue'
import type { CollectionIsolationId, IsolationId } from '@/type/types'
import type { ReindexOperation } from '@/api/reindex'
import { useModalStore } from '@/store/modalStore'
import { useConfigStore } from '@/store/configStore'
import { useMessageStore } from '@/store/messageStore'
import { useSearchFacetStore } from '@/store/searchFacetStore'
import { useAlbumStore } from '@/store/albumStore'
import { useRerenderStore } from '@/store/rerenderStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import { useReindexJobs } from '@/script/hook/useReindexJobs'
import ReindexPlanForm from './ReindexPlanForm.vue'
import ReindexJobList from './ReindexJobList.vue'

const modalStore = useModalStore('mainId')
const configStore = useConfigStore('mainId')
const messageStore = useMessageStore('mainId')
const searchFacetStore = useSearchFacetStore()
const rerenderStore = useRerenderStore('mainId')
const albumStores: Record<CollectionIsolationId, ReturnType<typeof useAlbumStore>> = {
  mainId: useAlbumStore('mainId'),
  subId: useAlbumStore('subId'),
  tempId: useAlbumStore('tempId'),
  shareId: useAlbumStore('shareId')
}

const activeTab = shallowRef<'create' | 'queue'>('create')
const formKey = shallowRef(0)
const context = computed(() => modalStore.reindexContext)
const isOpen = computed({
  get: () => context.value !== null,
  set: (value: boolean) => {
    if (!value) modalStore.closeReindex()
  }
})

const handleTerminalSuccess = async (_job: unknown, isolationId: IsolationId) => {
  await tryWithMessageStore('mainId', async () => {
    const albumIsolationId: CollectionIsolationId =
      isolationId === 'detailId' || isolationId === 'subDetailId' ? 'mainId' : isolationId
    searchFacetStore.clearAll()
    for (const albumStore of Object.values(albumStores)) albumStore.clearAll()
    await Promise.all([
      searchFacetStore.fetchFacets(),
      albumStores[albumIsolationId].fetchAlbums()
    ])
    if (isolationId === 'subId') rerenderStore.rerenderHomeIsolated()
    else rerenderStore.rerenderHome()
  })
}

const {
  jobs,
  activeJobs,
  loading,
  submitting,
  cancelingJobIds,
  refreshJobs,
  startPolling,
  submit,
  cancel
} = useReindexJobs({ onTerminalSuccess: handleTerminalSuccess })

const createJob = async (operations: ReindexOperation[]) => {
  const requestContext = context.value
  if (requestContext === null) return
  const accepted = await tryWithMessageStore('mainId', () => submit(requestContext, operations))
  if (accepted === undefined) return
  activeTab.value = 'queue'
  messageStore.success(
    `Reindex job queued for ${accepted.targetCount.toLocaleString()} targets`
  )
}

const cancelJob = async (jobId: string) => {
  const result = await tryWithMessageStore('mainId', async () => {
    await cancel(jobId)
    return true
  })
  if (result === true) messageStore.info('Cancel requested')
}

watch(context, (nextContext) => {
  if (nextContext === null) return
  activeTab.value = 'create'
  formKey.value += 1
  startPolling()
  void tryWithMessageStore('mainId', refreshJobs)
})
</script>

<template>
  <v-dialog
    id="reindex-overlay"
    v-model="isOpen"
    :max-width="780"
    :fullscreen="configStore.isMobile"
    persistent
    scrollable
  >
    <v-card border flat rounded="lg" color="surface" class="d-flex flex-column">
      <v-card-item title="Reindex media" class="font-weight-bold">
        <template #append>
          <v-btn
            icon="mdi-close"
            aria-label="Close reindex dialog"
            variant="text"
            density="comfortable"
            :disabled="submitting"
            @click="isOpen = false"
          />
        </template>
      </v-card-item>

      <v-progress-linear v-if="submitting" indeterminate color="primary" height="2" />
      <v-divider v-else thickness="4" variant="double" />

      <v-tabs
        v-model="activeTab"
        color="primary"
        grow
        class="flex-grow-0 flex-shrink-0"
      >
        <v-tab value="create" prepend-icon="mdi-playlist-plus" text="Create job" />
        <v-tab value="queue" prepend-icon="mdi-format-list-checks">
          Job queue
          <v-badge
            v-if="activeJobs.length > 0"
            :content="activeJobs.length"
            color="primary"
            inline
            class="ms-2"
          />
        </v-tab>
      </v-tabs>
      <v-divider />

      <v-card-text class="pa-0 flex-grow-1 flex-shrink-1 overflow-y-auto">
        <v-window v-model="activeTab">
          <v-window-item value="create">
            <v-container fluid class="pa-4 pa-sm-6">
              <ReindexPlanForm
                v-if="context !== null"
                :key="formKey"
                :target-count="context.targetCount"
                :submitting="submitting"
                @submit="createJob"
              />
            </v-container>
          </v-window-item>

          <v-window-item value="queue">
            <v-container fluid class="pa-4 pa-sm-6">
              <ReindexJobList
                :jobs="jobs"
                :loading="loading"
                :canceling-job-ids="cancelingJobIds"
                @cancel="cancelJob"
              />
            </v-container>
          </v-window-item>
        </v-window>
      </v-card-text>
    </v-card>
  </v-dialog>
</template>
