<template>
  <BaseModal
    v-model="isOpen"
    title="Reindex media"
    :width="780"
    :fullscreen="configStore.isMobile"
    :loading="submitting"
    content-class="pa-0"
    id="reindex-overlay"
  >
    <v-tabs v-model="activeTab" color="primary" grow>
      <v-tab value="create" prepend-icon="mdi-playlist-plus">Create job</v-tab>
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

    <v-window v-model="activeTab">
      <v-window-item value="create">
        <div class="pa-4 pa-sm-6">
          <ReindexPlanForm
            v-if="context !== null"
            :key="formKey"
            :target-count="context.targetCount"
            :submitting="submitting"
            @submit="createJob"
          />
        </div>
      </v-window-item>
      <v-window-item value="queue">
        <div class="pa-4 pa-sm-6">
          <ReindexJobList
            :jobs="jobs"
            :loading="loading"
            :canceling-job-ids="cancelingJobIds"
            @cancel="cancelJob"
          />
        </div>
      </v-window-item>
    </v-window>
  </BaseModal>
</template>

<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue'
import type { IsolationId } from '@/type/types'
import type { ReindexOperation } from '@/api/reindex'
import { useModalStore } from '@/store/modalStore'
import { useConfigStore } from '@/store/configStore'
import { useMessageStore } from '@/store/messageStore'
import { useSearchFacetStore } from '@/store/searchFacetStore'
import { useAlbumStore } from '@/store/albumStore'
import { useRerenderStore } from '@/store/rerenderStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import { useReindexJobs } from '@/script/hook/useReindexJobs'
import BaseModal from '../BaseModal.vue'
import ReindexPlanForm from './ReindexPlanForm.vue'
import ReindexJobList from './ReindexJobList.vue'

const modalStore = useModalStore('mainId')
const configStore = useConfigStore('mainId')
const messageStore = useMessageStore('mainId')
const searchFacetStore = useSearchFacetStore()
const rerenderStore = useRerenderStore('mainId')
const albumStores: Record<IsolationId, ReturnType<typeof useAlbumStore>> = {
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
    searchFacetStore.clearAll()
    for (const albumStore of Object.values(albumStores)) albumStore.clearAll()
    await Promise.all([searchFacetStore.fetchFacets(), albumStores[isolationId].fetchAlbums()])
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
