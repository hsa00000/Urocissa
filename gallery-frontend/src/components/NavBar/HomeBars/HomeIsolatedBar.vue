<template>
  <HomeBarTemplate isolation-id="subId">
    <template #content>
      <v-toolbar v-if="!collectionStore.editModeOn" class="position-relative bg-surface">
        <LeaveView />

        <v-card elevation="0" class="title-card">
          <v-card-title>
            <v-text-field
              class="album-title-field"
              data-testid="album-title"
              v-model="titleModel"
              variant="plain"
              @blur="editTitle(props.album, titleModel)"
              :placeholder="titleModel === '' ? 'Untitled' : undefined"
            />
          </v-card-title>
        </v-card>

        <v-card elevation="0" class="search-card" v-if="false">
          <v-card-text class="pa-0">
            <v-text-field
              id="nav-search-input-isolated"
              v-model="searchQuery"
              rounded
              class="ma-0 mr-2"
              bg-color="surface-light"
              @click:prepend-inner="handleSearch"
              @click:clear="handleSearch"
              @keyup.enter="handleSearch"
              clearable
              persistent-clear
              variant="solo"
              flat
              prepend-inner-icon="mdi-magnify"
              single-line
              hide-details
            >
              <template #label>
                <span class="text-body-small">Search</span>
              </template>
            </v-text-field>
          </v-card-text>
        </v-card>
        <v-spacer></v-spacer>
        <v-btn
          icon="mdi-share-variant"
          aria-label="Share album"
          @click="modalStore.showShareModal = true"
        ></v-btn>
        <v-btn
          icon="mdi-image-plus"
          aria-label="Add album items"
          @click="modalStore.showHomeTempModal = true"
        ></v-btn>
      </v-toolbar>

      <EditBar v-if="collectionStore.editModeOn" />

      <HomeTemp v-if="modalStore.showHomeTempModal" :album="props.album"> </HomeTemp>
      <CreateShareModal
        v-if="modalStore.showShareModal"
        :album-id="props.album.id"
        :mode="'create'"
      />
    </template>
  </HomeBarTemplate>
</template>

<script setup lang="ts">
import { useCollectionStore } from '@/store/collectionStore'
import LeaveView from '@/components/Menu/MenuButton/BtnLeaveView.vue'
import EditBar from '@/components/NavBar/EditBar.vue'
import HomeTemp from '@/components/Home/HomeTemp.vue'
import CreateShareModal from '@/components/Modal/CreateShareModal.vue'
import HomeBarTemplate from '@/components/NavBar/HomeBars/HomeBarTemplate.vue'
import { GalleryAlbum } from '@type/types'
import { useModalStore } from '@/store/modalStore'
import { Ref, ref, watch, watchEffect } from 'vue'
import { editTitle } from '@utils/createAlbums'
import { LocationQueryValue, useRoute, useRouter } from 'vue-router'
import { useFilterStore } from '@/store/filterStore'

const props = defineProps<{
  album: GalleryAlbum
}>()

const modalStore = useModalStore('mainId')
const collectionStore = useCollectionStore('subId')
const filterStore = useFilterStore('subId')

const route = useRoute()
const router = useRouter()

const titleModel = ref('')

const searchQuery: Ref<LocationQueryValue | LocationQueryValue[] | undefined> = ref(null)

const handleSearch = async () => {
  filterStore.searchString = searchQuery.value

  const nextQuery = { ...route.query }

  // remove key when cleared
  const v = searchQuery.value
  if (v === null || v === undefined || v === '') {
    delete nextQuery.subSearch
  } else {
    nextQuery.subSearch = v
  }

  await router.replace({
    path: route.path,
    query: nextQuery
  })
}

watch(
  () => props.album.title,
  () => {
    titleModel.value = props.album.title ?? ''
  },
  { immediate: true }
)

watchEffect(() => {
  searchQuery.value = filterStore.searchString
})
</script>

<style scoped>
.album-title-field :deep(input) {
  font-size: 22px;
  font-weight: 400;
  line-height: 1.175;
  letter-spacing: 0.0073529412em;
  margin-bottom: -8px;
}

.title-card {
  flex: 0 1 420px; /* fixed-ish */
  min-width: 240px;
}

.search-card {
  flex: 1 1 auto; /* takes remaining space up to the buttons */
  min-width: 260px;
}
</style>
