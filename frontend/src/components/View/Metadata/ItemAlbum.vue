<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon >mdi-image-album</v-icon>
      </v-avatar>
    </template>
    <v-list-item-subtitle class="text-wrap">
      <v-chip
        variant="flat"
        color="primary"
        v-for="albumId in props.albums"
        :key="albumId"
        link
        class="ma-1"
        @click="navigateToAlbum(albumId, router)"
      >
        {{ albumStore.albums.get(albumId)?.displayName }}
      </v-chip>
    </v-list-item-subtitle>
    <v-list-item-subtitle>
      <v-chip
        prepend-icon="mdi-pencil"
        color="surface-variant"
        variant="outlined"
        class="ma-1"
        link
        @click="openEditAlbumsModal"
        >edit</v-chip
      >
    </v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useAlbumStore } from '@/store/albumStore'
import { IsolationId } from '@type/types'
import { navigateToAlbum } from '@/route/navigator'

const props = defineProps<{
  isolationId: IsolationId
  index: number
  albums: string[]
}>()

const modalStore = useModalStore('mainId')
const albumStore = useAlbumStore('mainId')
const router = useRouter()

function openEditAlbumsModal() {
  modalStore.showEditAlbumsModal = true
}
</script>
