<template>
  <PageTemplate preset="card" width="pane" :ready="searchFacetStore.fetched">
    <template #content>
      <v-table hover>
        <thead>
          <tr>
            <th>tag</th>
            <th>number of items</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="tag in searchFacetStore.tags" :key="tag.value">
            <td class="key-cell">
              <v-btn
                @click="searchByTag(tag.value, router)"
                slim
                class="text-body-small"
                variant="tonal"
              >
                {{ tag.value }}
              </v-btn>
            </td>
            <td>{{ tag.count }}</td>
          </tr>
        </tbody>
      </v-table>
    </template>
  </PageTemplate>
</template>

<script setup lang="ts">
import { onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useSearchFacetStore } from '@/store/searchFacetStore'
import { useInitializedStore } from '@/store/initializedStore'
import { searchByTag } from '@utils/getter'
import PageTemplate from './PageLayout/PageTemplate.vue'

const initializedStore = useInitializedStore('mainId')
const searchFacetStore = useSearchFacetStore()
const router = useRouter()

onMounted(async () => {
  if (!searchFacetStore.fetched) {
    await searchFacetStore.fetchFacets()
  }
  initializedStore.initialized = true
})

onBeforeUnmount(() => {
  initializedStore.initialized = false
})
</script>
