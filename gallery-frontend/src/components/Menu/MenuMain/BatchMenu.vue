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

import ItemSetAsCover from '@Menu/MenuItem/ItemSetAsCover.vue'
import ItemArchive from '@Menu/MenuItem/ItemArchive.vue'
import ItemFavorite from '@Menu/MenuItem/ItemFavorite.vue'
import ItemBatchEditTags from '@Menu/MenuItem/ItemBatchEditTags.vue'
import ItemBatchEditAlbums from '@Menu/MenuItem/ItemBatchEditAlbums.vue'
import ItemDownload from '@Menu/MenuItem/ItemDownload.vue'
import ItemDelete from '@Menu/MenuItem/ItemDelete.vue'
import ItemPermanentlyDelete from '@Menu/MenuItem/ItemPermanentlyDelete.vue'
import ItemRegenerateMetadata from '@Menu/MenuItem/ItemRegenerateMetadata.vue'
import ItemRestore from '@Menu/MenuItem/ItemRestore.vue'

import { getIsolationIdByRoute } from '@utils/getter'

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const collectionStore = useCollectionStore(isolationId)

const editModeList = computed(() => Array.from(collectionStore.editModeCollection))

const shouldShowSetAsCover = computed(
  () => route.meta.isReadPage && collectionStore.editModeCollection.size === 1
)

const isInTrashedPath = computed(() => route.meta.baseName === 'trashed')
</script>
