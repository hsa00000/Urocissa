<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useSearchFacetStore } from '@/store/searchFacetStore'
import { useInitializedStore } from '@/store/initializedStore'
import { searchByFacet, type FacetSearchScope, type FacetSearchField } from '@utils/getter'
import type { SearchFacets } from '@type/types'
import FacetTable from './TagsPage/FacetTable.vue'
import PageTemplate from './PageLayout/PageTemplate.vue'

interface ScopeState {
  data: SearchFacets | null
  loading: boolean
  error: boolean
  pending: Promise<void> | null
}

const initializedStore = useInitializedStore('mainId')
const searchFacetStore = useSearchFacetStore()
const route = useRoute()
const router = useRouter()

const scopeState = reactive<Record<FacetSearchScope, ScopeState>>({
  all: { data: null, loading: false, error: false, pending: null },
  trashed: { data: null, loading: false, error: false, pending: null }
})

const selectedScope = computed<FacetSearchScope>(() =>
  route.query.scope === 'trashed' ? 'trashed' : 'all'
)
const currentState = computed(() => scopeState[selectedScope.value])
const currentFacets = computed(() => currentState.value.data)

function normalizeScopeQuery(): void {
  const rawScope = route.query.scope
  if (rawScope === undefined || rawScope === 'trashed') return

  const query = { ...route.query }
  delete query.scope
  void router.replace({ path: route.path, query })
}

function setScope(value: unknown): void {
  if (value !== 'all' && value !== 'trashed') return

  const query = { ...route.query }
  if (value === 'trashed') query.scope = value
  else delete query.scope

  void router.replace({ path: route.path, query })
}

async function ensureScopeLoaded(scope: FacetSearchScope, force = false): Promise<void> {
  const state = scopeState[scope]
  if (!force && state.data !== null) return
  if (state.pending !== null) return state.pending

  state.data = null
  state.loading = true
  state.error = false

  const pending = (async () => {
    try {
      const facets = await searchFacetStore.fetchFacetsForTrashState(scope === 'trashed')
      if (facets === undefined) state.error = true
      else state.data = facets
    } catch {
      state.error = true
    } finally {
      state.loading = false
      state.pending = null
    }
  })()

  state.pending = pending
  await pending
}

function retryCurrentScope(): void {
  void ensureScopeLoaded(selectedScope.value, true)
}

function searchFacet(field: FacetSearchField, value: string): void {
  void searchByFacet(field, value, router, selectedScope.value)
}

watch(
  () => route.query.scope,
  () => {
    normalizeScopeQuery()
  },
  { immediate: true }
)

watch(
  selectedScope,
  (scope) => {
    void ensureScopeLoaded(scope)
  },
  { immediate: true }
)

onMounted(async () => {
  if (!searchFacetStore.fetched) await searchFacetStore.fetchFacets()
  initializedStore.initialized = true
})

onBeforeUnmount(() => {
  initializedStore.initialized = false
})
</script>

<template>
  <PageTemplate preset="card" width="pane">
    <template #content>
      <v-card-text class="pb-2 d-flex flex-column align-center">
        <v-label
          for="facet-scope-toggle"
          text="Search scope"
          class="text-subtitle-2 mb-2"
        />
        <v-btn-toggle
          id="facet-scope-toggle"
          :model-value="selectedScope"
          mandatory
          color="primary"
          density="compact"
          border
          divided
          rounded="lg"
          aria-label="Facet search scope"
          @update:model-value="setScope"
        >
          <v-btn value="all" width="112">All</v-btn>
          <v-btn value="trashed" width="112">Trashed</v-btn>
        </v-btn-toggle>
      </v-card-text>

      <v-row class="ma-0" density="comfortable">
        <v-col v-if="currentState.loading" cols="12" class="pt-0">
          <v-progress-linear indeterminate color="primary" aria-label="Loading facets" />
        </v-col>

        <v-col v-else-if="currentState.error" cols="12">
          <v-alert type="error" variant="tonal" title="Unable to load facets">
            Please try again.
            <template #append>
              <v-btn color="primary" variant="text" @click="retryCurrentScope">Retry</v-btn>
            </template>
          </v-alert>
        </v-col>

        <template v-else-if="currentFacets !== null">
          <v-col cols="12">
            <FacetTable
              value-label="Tag"
              :items="currentFacets.tags"
              empty-text="No tags available"
              @select="searchFacet('tag', $event)"
            />
          </v-col>

          <v-col cols="12">
            <FacetTable
              value-label="Make"
              :items="currentFacets.makes"
              empty-text="No camera makes available"
              @select="searchFacet('make', $event)"
            />
          </v-col>

          <v-col cols="12">
            <FacetTable
              value-label="Model"
              :items="currentFacets.models"
              empty-text="No camera models available"
              @select="searchFacet('model', $event)"
            />
          </v-col>
        </template>
      </v-row>
    </template>
  </PageTemplate>
</template>
