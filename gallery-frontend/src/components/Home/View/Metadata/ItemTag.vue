<template>
  <v-list-item>
    <template #prepend>
      <v-avatar>
        <v-icon color="black">mdi-tag</v-icon>
      </v-avatar>
    </template>
    <v-list-item-title>
      <v-chip
        v-if="tags.includes('_favorite')"
        prepend-icon="mdi-star"
        color="black"
        variant="tonal"
        class="ma-1"
        link
        @click="quickRemoveTags('_favorite', [index], isolationId)"
        >favorite</v-chip
      >
      <v-chip
        v-else
        prepend-icon="mdi-star-outline"
        color="grey"
        variant="tonal"
        class="ma-1"
        link
        @click="quickAddTags('_favorite', [index], isolationId)"
        >favorite</v-chip
      >
      <v-chip
        v-if="tags.includes('_archived')"
        prepend-icon="mdi-archive-arrow-down"
        color="black"
        variant="tonal"
        class="ma-1"
        link
        @click="quickRemoveTags('_archived', [index], isolationId)"
        >archived</v-chip
      >
      <v-chip
        v-else
        prepend-icon="mdi-archive-arrow-down"
        color="grey"
        variant="tonal"
        class="ma-1"
        link
        @click="quickAddTags('_archived', [index], isolationId)"
        >archived</v-chip
      >
    </v-list-item-title>
    <v-list-item-subtitle class="text-wrap">
      <v-chip
        variant="flat"
        color="black"
        v-for="tag in filteredTags"
        :key="tag"
        link
        class="ma-1"
        @click="searchByTag(tag, router)"
      >
        {{ tag }}
      </v-chip>
    </v-list-item-subtitle>
    <v-list-item-subtitle>
      <v-chip
        prepend-icon="mdi-pencil"
        color="black"
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
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { IsolationId } from '@/script/common/types'
import { searchByTag } from '@utils/getter'
import { quickRemoveTags, quickAddTags } from '@/script/common/quickEditTags'

const props = defineProps<{
  isolationId: IsolationId
  index: number
  tags: string[]
}>()

const modalStore = useModalStore('mainId')

const router = useRouter()

// Computed Properties
const filteredTags = computed(() => {
  return props.tags.filter(
    (tag) => tag !== '_favorite' && tag !== '_archived' && tag !== '_trashed'
  )
})

function openEditTagsModal() {
  modalStore.showEditTagsModal = true
}
</script>
