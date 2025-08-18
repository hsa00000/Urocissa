<template>
  <!-- Root is a single v-col so parent v-row receives a valid column child -->
  <router-view v-slot="{ Component }">
    <component :is="Component" />
  </router-view>
  <v-col
    :class="[
      'd-flex',
      'align-center',
      'justify-center',
      colWidth < colHeight ? 'flex-column' : 'flex-row'
    ]"
  >
    <v-card
      :class="sizeClass"
      :style="{
        aspectRatio: '1',
        maxWidth: '500px',
        maxHeight: '500px',
        border: '8px solid white'
      }"
    >
      <img
        v-if="imgStore.imgOriginal.get(index)"
        id="album-img"
        :key="index"
        rounded="xl"
        aspect-ratio="1"
        class="h-100 w-100"
        cover
        :src="imgStore.imgOriginal.get(index)"
        :style="{
          objectFit: 'cover'
        }"
      />
    </v-card>
    <v-card
      :style="{
        aspectRatio: '1 / 1',
        maxWidth: '500px',
        maxHeight: '500px'
      }"
      outlined
      style="padding: 16px"
      :class="sizeClass"
      class="d-flex flex-column"
    >
      <v-card-item>
        <v-text-field
          v-model="titleModel"
          variant="underlined"
          @blur="editTitle(props.album, titleModel)"
          :placeholder="titleModel === '' ? 'Add Title' : undefined"
        ></v-text-field>
      </v-card-item>
      <v-list>
        <v-list-item>
          <v-list-item-title v-if="album.startTime">
            {{ `${dater(album.startTime)} ~ ${dater(album.endTime!)}` }}
          </v-list-item-title>
          <v-list-item-subtitle>
            {{ `${album.itemCount} item${album.itemCount === 1 ? '' : 's'}` }}
            •
            {{ filesize(album.itemSize) }}
          </v-list-item-subtitle>
        </v-list-item>
      </v-list>

      <!-- Use this div to take up remaining space -->
      <div class="flex-grow-1"></div>

      <v-card-actions class="justify-end" v-if="route.meta.level === 2">
        <v-btn
          color="teal-accent-4"
          variant="flat"
          class="ma-2 button button-submit"
          :to="route.meta.getChildPage(route, undefined)"
          @click="
            () => {
              albumStore.leaveAlbumPath = route.fullPath
            }
          "
        >
          Enter Album
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-col>
</template>

<script setup lang="ts">
import { useImgStore } from '@/store/imgStore'
import { useAlbumStore } from '@/store/albumStore'
import { VCol } from 'vuetify/components'
import { filesize } from 'filesize'
import { useRoute } from 'vue-router'
import { dater } from '@utils/dater'
import { Album } from '@type/types'
import { computed, ref, watch } from 'vue'
import { editTitle } from '@utils/createAlbums'

const titleModel = ref('')

const route = useRoute()

const albumStore = useAlbumStore('mainId')
const imgStore = useImgStore('mainId')

const props = defineProps<{
  index: number
  album: Album
  colWidth: number
  colHeight: number
}>()

const sizeClass = computed(() => {
  const w = props.colWidth
  const h = props.colHeight

  if (2 * w < h) return 'w-100'
  if (w < h && h < 2 * w) return 'h-50'
  if (h < w && w < 2 * h) return 'w-50'
  if (2 * h < w) return 'h-100'

  // boundaries (==) fallback
  return w <= h ? 'h-50' : 'w-50'
})

watch(
  () => props.album.title,
  () => {
    titleModel.value = props.album.title ?? ''
  },
  { immediate: true }
)
</script>
<style scoped>
.v-text-field :deep(input) {
  font-size: 2.125rem;
  font-weight: 400;
  line-height: 1.175;
  letter-spacing: 0.0073529412em;
}
</style>
