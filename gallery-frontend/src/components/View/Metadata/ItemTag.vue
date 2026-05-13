<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-tag</v-icon>
      </v-avatar>
    </template>
    <div class="d-flex w-100 flex-wrap align-center ga-1 py-1">
      <v-chip
        v-if="route.meta.baseName !== 'share' && isFavorite"
        prepend-icon="mdi-star"
        color="warning"
        variant="tonal"
        :size="metadataChipSize"
        link
        @click="setFavorite([index], false, isolationId)"
        >favorite</v-chip
      >
      <v-chip
        v-else-if="route.meta.baseName !== 'share'"
        prepend-icon="mdi-star-outline"
        color="grey"
        variant="tonal"
        :size="metadataChipSize"
        link
        @click="setFavorite([index], true, isolationId)"
        >favorite</v-chip
      >
      <v-chip
        v-if="route.meta.baseName !== 'share' && isArchived"
        prepend-icon="mdi-archive-arrow-down"
        color="primary"
        variant="tonal"
        :size="metadataChipSize"
        link
        @click="setArchived([index], false, isolationId)"
        >archived</v-chip
      >
      <v-chip
        v-else-if="route.meta.baseName !== 'share'"
        prepend-icon="mdi-archive-arrow-down"
        color="grey"
        variant="tonal"
        :size="metadataChipSize"
        link
        @click="setArchived([index], true, isolationId)"
        >archived</v-chip
      >
      <v-chip
        variant="flat"
        color="primary"
        v-for="tag in tags"
        :key="tag"
        link
        :size="metadataChipSize"
        @click="searchByTag(tag, router)"
      >
        {{ tag }}
      </v-chip>
      <v-chip
        v-if="route.meta.baseName !== 'share'"
        prepend-icon="mdi-pencil"
        color="surface-variant"
        variant="outlined"
        :size="metadataChipSize"
        link
        @click="openEditTagsModal"
        >edit</v-chip
      >
    </div>
  </v-list-item>
</template>

<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import type { IsolationId } from '@type/types'
import { searchByTag } from '@utils/getter'
import { setFavorite, setArchived } from '@/api/editFlags'
import { useMetadataItemLayout } from './useMetadataItemLayout'

// eslint-disable-next-line @typescript-eslint/no-unused-vars
const props = defineProps<{
  isolationId: IsolationId
  index: number
  tags: string[]
  isFavorite: boolean
  isArchived: boolean
}>()

const modalStore = useModalStore('mainId')

const route = useRoute()
const router = useRouter()
const { metadataChipSize } = useMetadataItemLayout()

function openEditTagsModal() {
  modalStore.showEditTagsModal = true
}
</script>
