<template>
  <v-overlay
    v-model="modalStore.showHomeTempModal"
    :height="'100%'"
    :width="'100%'"
    class="d-flex"
    id="view-page"
    transition="false"
    :close-on-back="false"
  >
    <Home isolation-id="tempId" :basic-string="basicString" :search-string="null">
      <template #home-toolbar>
        <HomeTempBar :album="album" />
      </template>
    </Home>
  </v-overlay>
</template>
<script setup lang="ts">
import { GalleryAlbum } from '@type/types'
import Home from './Home.vue'
import HomeTempBar from '@/components/NavBar/HomeBars/HomeTempBar.vue'
import { useModalStore } from '@/store/modalStore'
import { onBeforeRouteLeave } from 'vue-router'
const modalStore = useModalStore('mainId')
const props = defineProps<{
  album: GalleryAlbum
}>()

const basicString = `and(not(type:"album"), trashed:false, not(album:"${props.album.id}"))`
onBeforeRouteLeave(() => {
  // 如果 overlay 正在顯示，就先關掉並阻止這次導航
  if (modalStore.showHomeTempModal) {
    modalStore.showHomeTempModal = false
    return false // 取消這次 route 切換（包含瀏覽器上一頁）
  }
  // 若 overlay 已經關閉，直接允許導航（什麼都不 return 就是放行）
})
</script>
