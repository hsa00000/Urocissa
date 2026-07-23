<script setup lang="ts">
import { computed, inject, onMounted, type Ref } from 'vue'
import { useRoute } from 'vue-router'
import SavedSearchSection from '@/components/SavedSearch/SavedSearchSection.vue'
import { useInitializedStore } from '@/store/initializedStore'
import { useModalStore } from '@/store/modalStore'
import { useSavedSearchStore } from '@/store/savedSearchStore'

const showDrawer = inject<Ref<boolean>>('showDrawer')
const route = useRoute()
const modalStore = useModalStore('mainId')
const initializedStore = useInitializedStore('mainId')
const savedSearchStore = useSavedSearchStore()
const hasSavedSearches = computed(() => savedSearchStore.searches.length > 0)

onMounted(() => {
  void savedSearchStore.loadOnce()
})
</script>

<template>
  <v-navigation-drawer v-model="showDrawer" temporary touchless width="220" class="no-select">
    <v-list :key="route.fullPath" nav :disabled="!initializedStore.initialized">
      <v-list-item slim to="/home" prepend-icon="mdi-home" title="Home"></v-list-item>
      <v-divider></v-divider>
      <v-list-item slim to="/favorite" prepend-icon="mdi-star" title="Favorite"></v-list-item>
      <v-list-item
        slim
        to="/archived"
        prepend-icon="mdi-archive-arrow-down"
        title="Archived"
      ></v-list-item>
      <v-list-item slim to="/trashed" prepend-icon="mdi-trash-can" title="Trashed"></v-list-item>
      <v-list-item slim to="/all" prepend-icon="mdi-expand-all" title="All"></v-list-item>
      <v-divider></v-divider>
      <v-list-item slim to="/albums" prepend-icon="mdi-image-album" title="Albums"></v-list-item>
      <v-list-item
        slim
        to="/videos"
        prepend-icon="mdi-play-circle-outline"
        title="Videos"
      ></v-list-item>
      <v-divider></v-divider>
      <v-list-item slim to="/tags" prepend-icon="mdi-tag-multiple" title="Tags"></v-list-item>
      <v-list-item slim to="/links" prepend-icon="mdi-link" title="Links"></v-list-item>
    </v-list>

    <v-divider />
    <SavedSearchSection
      v-if="hasSavedSearches"
      :disabled="!initializedStore.initialized"
    />
    <v-divider v-if="hasSavedSearches" />

    <v-list nav :disabled="!initializedStore.initialized">
      <v-list-item
        slim
        prepend-icon="mdi-cog-outline"
        title="Settings"
        @click="modalStore.showSettingModal = true"
      ></v-list-item>
    </v-list>

    <template #append>
      <v-list nav :key="route.fullPath" :disabled="!initializedStore.initialized">
        <v-list-item slim to="/config" prepend-icon="mdi-tune" title="Config"></v-list-item>
      </v-list>
    </template>
  </v-navigation-drawer>
</template>
