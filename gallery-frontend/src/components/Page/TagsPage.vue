<script setup lang="ts">
import { onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useSearchFacetStore } from '@/store/searchFacetStore'
import { useInitializedStore } from '@/store/initializedStore'
import { searchByFacet, type FacetSearchField } from '@utils/getter'
import FacetTable from './TagsPage/FacetTable.vue'
import PageTemplate from './PageLayout/PageTemplate.vue'

const initializedStore = useInitializedStore('mainId')
const searchFacetStore = useSearchFacetStore()
const router = useRouter()

function searchFacet(field: FacetSearchField, value: string): void {
  void searchByFacet(field, value, router)
}

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

<template>
  <PageTemplate preset="card" width="wide" :ready="searchFacetStore.fetched">
    <template #content>
      <v-row class="ma-0" density="comfortable">
        <v-col cols="12" md="4">
          <FacetTable
            value-label="Tag"
            :items="searchFacetStore.tags"
            empty-text="No tags available"
            @select="searchFacet('tag', $event)"
          />
        </v-col>

        <v-col cols="12" md="4">
          <FacetTable
            value-label="Make"
            :items="searchFacetStore.makes"
            empty-text="No camera makes available"
            @select="searchFacet('make', $event)"
          />
        </v-col>

        <v-col cols="12" md="4">
          <FacetTable
            value-label="Model"
            :items="searchFacetStore.models"
            empty-text="No camera models available"
            @select="searchFacet('model', $event)"
          />
        </v-col>
      </v-row>
    </template>
  </PageTemplate>
</template>
