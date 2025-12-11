<template>
  <HomeBarTemplate isolation-id="subId">
    <template #content>
      <v-toolbar v-if="!collectionStore.editModeOn" class="position-relative bg-surface">
        <LeaveView />
        <v-card elevation="0" class="w-25">
          <v-card-title>
            <v-text-field
              v-model="titleModel"
              variant="plain"
              class="title-input"
              @blur="editTitle(props.album, titleModel)"
              :placeholder="titleModel === '' ? 'Untitled' : undefined"
            ></v-text-field>
          </v-card-title>
        </v-card>

        <v-card elevation="0" class="w-50">
          <v-card-text class="pa-0">
            <v-text-field
              id="nav-search-input"
              rounded
              class="ma-0"
              v-model="searchQuery"
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
              style="margin-right: 10px"
            >
              <template #label>
                <span class="text-caption">Search</span>
              </template>
            </v-text-field>
          </v-card-text>
        </v-card>

        <v-spacer></v-spacer>
        <v-btn icon="mdi-share-variant" @click="modalStore.showShareModal = true"> </v-btn>
        <v-btn icon="mdi-image-plus" @click="modalStore.showHomeTempModal = true"> </v-btn>
      </v-toolbar>
      <EditBar v-else />
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
import { Album } from '@type/types'
import { useModalStore } from '@/store/modalStore'
import { Ref, ref, watch, watchEffect } from 'vue'
import { editTitle } from '@utils/createAlbums'
import { LocationQueryValue, useRoute, useRouter } from 'vue-router'
import { useFilterStore } from '@/store/filterStore'

const props = defineProps<{
  album: Album
}>()

const modalStore = useModalStore('mainId')
const collectionStore = useCollectionStore('subId')
const filterStore = useFilterStore('subId')

const route = useRoute()
const router = useRouter()
const searchQuery: Ref<LocationQueryValue | LocationQueryValue[] | undefined> = ref(null)

const titleModel = ref('')

const handleSearch = async () => {
  await router.replace({
    path: route.path,
    query: { ...route.query, asearch: searchQuery.value }
  })
}

watchEffect(() => {
  searchQuery.value = filterStore.searchString
})

watch(
  () => props.album.title,
  () => {
    titleModel.value = props.album.title ?? ''
  },
  { immediate: true }
)
</script>

<style scoped>
/* Change selector to target only .title-input */
.title-input :deep(input) {
  font-size: 22px;
  font-weight: 400;
  line-height: 1.175;
  letter-spacing: 0.0073529412em;
  margin-bottom: -8px;
}

/* Remove the override for nav-search-input since it will now use defaults */
</style>
