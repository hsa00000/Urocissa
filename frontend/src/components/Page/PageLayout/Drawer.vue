<template>
  <v-navigation-drawer v-model="showDrawer" temporary touchless width="150" class="no-select">
    <v-list nav :key="route.fullPath" :disabled="!initializedStore.initialized">
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
      <v-divider></v-divider>
      <v-list-item
        slim
        @click="modalStore.showSettingModal = true"
        prepend-icon="mdi-cog-outline"
        title="UI Settings"
      ></v-list-item>
      <v-list-item
        slim
        @click="modalStore.showConfigModal = true"
        prepend-icon="mdi-tune"
        title="System Config"
      ></v-list-item>
    </v-list>
  </v-navigation-drawer>
</template>

<script setup lang="ts">
import { inject, type Ref } from 'vue'
import { useRoute } from 'vue-router'
import { useInitializedStore } from '@/store/initializedStore'
import { useModalStore } from '@/store/modalStore'

const showDrawer = inject<Ref<boolean>>('showDrawer')
const route = useRoute()
const modalStore = useModalStore('mainId')
const initializedStore = useInitializedStore('mainId')
</script>

<style scoped>
.no-select {
  user-select: none;
}
.no-select * {
  user-select: none;
}
</style>
