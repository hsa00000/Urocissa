<template>
  <HomeBarTemplate isolation-id="subId">
    <template #content>
      <v-toolbar v-if="!collectionStore.editModeOn" class="position-relative bg-surface">
        <LeaveView />
        <v-card elevation="0" class="w-100">
          <v-card-title>
            <v-text-field
              v-model="titleModel"
              variant="plain"
              @blur="editTitle(props.album, titleModel)"
              :placeholder="titleModel === '' ? 'Untitled' : undefined"
            ></v-text-field>
          </v-card-title>
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
import { GalleryAlbum } from '@type/types'
import { useModalStore } from '@/store/modalStore'
import { ref, watch } from 'vue'
import { editTitle } from '@utils/createAlbums'

const props = defineProps<{
  album: GalleryAlbum
}>()

const modalStore = useModalStore('mainId')
const collectionStore = useCollectionStore('subId')

const titleModel = ref('')

watch(
  () => props.album.title,
  () => {
    titleModel.value = props.album.title ?? ''
  },
  { immediate: true }
)
</script>

<style scoped>
.v-text-field :deep(input) {
  font-size: 22px;
  font-weight: 400;
  line-height: 1.175;
  letter-spacing: 0.0073529412em;
  margin-bottom: -8px;
}
</style>
