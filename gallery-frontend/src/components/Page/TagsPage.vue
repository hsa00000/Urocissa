<template>
  <PageTemplate>
    <template #content>
      <v-container
        v-if="tagStore.fetched"
        id="table-container"
        class="h-100 pa-1 bg-surface-light d-flex align-start"
        fluid
      >
        <v-row justify="center" class="ma-0 w-100">
          <v-col cols="12" sm="12" md="10" lg="8" class="d-flex justify-center">
            <v-card tile flat class="overflow-y-auto">
              <v-table hover>
                <thead ref="tableRef">
                  <tr>
                    <th>tag</th>
                    <th>number of items</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="tagsData in tagStore.tags" :key="tagsData.tag">
                    <td class="key-cell">
                      <v-btn
                        @click="searchByTag(tagsData.tag, router)"
                        slim
                        class="text-caption"
                        variant="tonal"
                      >
                        {{ tagsData.tag }}</v-btn
                      >
                    </td>
                    <td>{{ tagsData.number }}</td>
                  </tr>
                </tbody>
              </v-table>
            </v-card>
          </v-col>
        </v-row>
      </v-container>
    </template>
  </PageTemplate>
</template>
<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useTagStore } from '@/store/tagStore'
import { useInitializedStore } from '@/store/initializedStore'
import { onMounted } from 'vue'
import { onBeforeUnmount } from 'vue'
import { searchByTag } from '@utils/getter'
import PageTemplate from './PageLayout/PageTemplate.vue'
const initializedStore = useInitializedStore('mainId')
const tagStore = useTagStore('mainId')
const router = useRouter()
const dynamicWidth = ref<number>(0)
const tableRef = ref<HTMLElement | null>(null)
const updateDynamicWidth = () => {
  const tableWidth = tableRef.value?.offsetWidth ?? 0
  dynamicWidth.value = tableWidth <= 300 ? 300 : tableWidth
}

watch(
  () => initializedStore.initialized,
  () => {
    if (initializedStore.initialized) {
      updateDynamicWidth()
    }
  }
)

onMounted(async () => {
  if (!tagStore.fetched) {
    await tagStore.fetchTags()
  }
  initializedStore.initialized = true
})

onBeforeUnmount(() => {
  initializedStore.initialized = false
})
</script>

<style scoped>
#table-container {
  display: flex;
  justify-content: center;
  position: relative;
  padding: 4px;
  padding-top: 4px;
  background-color: #3d3d3d;
  overflow-y: scroll;
  height: 100dvh;
  width: 100%;
}
</style>
