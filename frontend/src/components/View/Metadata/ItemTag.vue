<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon>mdi-tag</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title v-if="route.meta.baseName !== 'share'">
      <v-chip
        v-if="isFavorite"
        prepend-icon="mdi-star"
        color="warning"
        variant="tonal"
        class="ma-1"
        link
        @click="setFavorite([index], false, isolationId)"
        >favorite</v-chip
      >
      <v-chip
        v-else
        prepend-icon="mdi-star-outline"
        color="grey"
        variant="tonal"
        class="ma-1"
        link
        @click="setFavorite([index], true, isolationId)"
        >favorite</v-chip
      >
      <v-chip
        v-if="isArchived"
        prepend-icon="mdi-archive-arrow-down"
        color="primary"
        variant="tonal"
        class="ma-1"
        link
        @click="setArchived([index], false, isolationId)"
        >archived</v-chip
      >
      <v-chip
        v-else
        prepend-icon="mdi-archive-arrow-down"
        color="grey"
        variant="tonal"
        class="ma-1"
        link
        @click="setArchived([index], true, isolationId)"
        >archived</v-chip
      >
    </v-list-item-title>
    <v-list-item-subtitle class="text-wrap">
      <v-chip
        variant="flat"
        color="primary"
        v-for="tag in tags"
        :key="tag"
        link
        class="ma-1"
        @click="searchByTag(tag, router)"
      >
        {{ tag }}
      </v-chip>
    </v-list-item-subtitle>
    <v-list-item-subtitle v-if="route.meta.baseName !== 'share'">
      <v-chip
        prepend-icon="mdi-pencil"
        color="surface-variant"
        variant="outlined"
        class="ma-1"
        link
        @click="openEditTagsModal"
        >edit</v-chip
      >
    </v-list-item-subtitle>
  </v-list-item>
</template>

<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { IsolationId } from '@type/types'
import { searchByTag } from '@utils/getter'
import { setFavorite, setArchived } from '@/api/editFlags'

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

function openEditTagsModal() {
  modalStore.showEditTagsModal = true
}
</script>
