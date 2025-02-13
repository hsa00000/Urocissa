<!-- NavBarAppBarEditBarMenuNormal.vue -->
<template>
  <v-menu>
    <template #activator="{ props: MenuBtn }">
      <v-btn v-bind="MenuBtn" icon="mdi-dots-vertical"></v-btn>
    </template>
    <v-list>
      <!-- Conditional Set as Cover -->
      <ItemSetAsCover v-if="shouldShowSetAsCover" />

      <v-divider v-if="shouldShowSetAsCover"></v-divider>

      <!-- Archive and Favorite Actions -->
      <ItemArchive :index-list="editModeList" />
      <ItemFavorite :index-list="editModeList" />
      <ItemBatchEditTags />
      <ItemBatchEditAlbums />

      <v-divider></v-divider>

      <!-- Download Action -->
      <ItemDownload :index-list="editModeList" />

      <v-divider></v-divider>

      <!-- Delete or Permanently Delete Actions -->
      <ItemDelete :index-list="editModeList" v-if="!isInTrashedPath" />
      <ItemRestore :index-list="editModeList" v-if="isInTrashedPath" />
      <ItemPermanentlyDelete :index-list="editModeList" v-if="isInTrashedPath" />

      <v-divider></v-divider>

      <!-- Regenerate Action -->
      <ItemRegenerateMetadata :index-list="editModeList" />
    </v-list>
  </v-menu>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useCollectionStore } from '@/store/collectionStore'

import ItemSetAsCover from './Item/ItemSetAsCover.vue'
import ItemArchive from './Item/ItemArchive.vue'
import ItemFavorite from './Item/ItemFavorite.vue'
import ItemBatchEditTags from './Item/ItemBatchEditTags.vue'
import ItemBatchEditAlbums from './Item/ItemBatchEditAlbums.vue'
import ItemDownload from './Item/ItemDownload.vue'
import ItemDelete from './Item/ItemDelete.vue'
import ItemPermanentlyDelete from './Item/ItemPermanentlyDelete.vue'
import ItemRegenerateMetadata from './Item/ItemRegenerateMetadata.vue'
import ItemRestore from './Item/ItemRestore.vue'

import { getIsolationIdByRoute } from '@/script/common/functions'

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const collectionStore = useCollectionStore(isolationId)

const editModeList = computed(() => Array.from(collectionStore.editModeCollection))

const shouldShowSetAsCover = computed(
  () => route.meta.isReadPage && collectionStore.editModeCollection.size === 1
)

const isInTrashedPath = computed(() => route.meta.baseName === 'trashed')
</script>
