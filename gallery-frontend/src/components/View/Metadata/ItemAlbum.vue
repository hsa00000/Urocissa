<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-image-album</v-icon>
      </v-avatar>
    </template>
    <div class="d-flex w-100 flex-wrap align-center ga-1 py-1">
      <v-chip
        variant="flat"
        color="primary"
        v-for="albumId in visibleAlbumIds"
        :key="albumId"
        link
        :size="metadataChipSize"
        @click="navigateToAlbum(albumId, router)"
      >
        {{ albumStore.albums.get(albumId)?.displayName }}
      </v-chip>
      <v-chip
        prepend-icon="mdi-pencil"
        color="surface-variant"
        variant="outlined"
        :size="metadataChipSize"
        link
        @click="openEditAlbumsModal"
        >edit</v-chip
      >
    </div>
  </v-list-item>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useAlbumStore } from '@/store/albumStore'
import type { IsolationId } from '@type/types'
import { navigateToAlbum } from '@/route/navigator'
import { useMetadataItemLayout } from './useMetadataItemLayout'

const props = defineProps<{
  isolationId: IsolationId
  index: number
  albums: string[]
}>()

const modalStore = useModalStore('mainId')
const albumStore = useAlbumStore('mainId')
const router = useRouter()
const { metadataChipSize } = useMetadataItemLayout()

const visibleAlbumIds = computed(() =>
  props.albums.filter((albumId) => albumStore.albums.has(albumId))
)

function openEditAlbumsModal() {
  modalStore.showEditAlbumsModal = true
}
</script>
