<template>
  <div class="h-100 d-flex align-center justify-center">
    <router-view v-slot="{ Component }">
      <component :is="Component" />
    </router-view>

    <div class="card-pair">
      <v-card class="square rounded-0" style="object-fit: cover; border: 8px solid #fff">
        <img
          v-if="imgStore.imgOriginal.get(index)"
          id="album-img"
          :key="index"
          :src="imgStore.imgOriginal.get(index)"
          class="w-100 h-100"
          style="object-fit: cover"
        />
      </v-card>

      <v-card class="square d-flex flex-column pa-4 rounded-0">
        <v-card-item>
          <v-text-field
            v-model="titleModel"
            variant="underlined"
            @blur="editTitle(props.album, titleModel)"
            :placeholder="titleModel === '' ? 'Add Title' : undefined"
          />
        </v-card-item>

        <v-list>
          <v-list-item>
            <v-list-item-title v-if="album.startTime">
              {{ `${dater(album.startTime)} ~ ${dater(album.endTime!)}` }}
            </v-list-item-title>
            <v-list-item-subtitle>
              {{ `${album.itemCount} item${album.itemCount === 1 ? '' : 's'}` }} •
              {{ filesize(album.itemSize) }}
            </v-list-item-subtitle>
          </v-list-item>
        </v-list>

        <div class="flex-grow-1" />

        <v-card-actions class="justify-end" v-if="route.meta.level === 2">
          <v-btn
            color="teal-accent-4"
            variant="flat"
            class="button button-submit"
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
    </div>
  </div>
</template>

<script setup lang="ts">
import { useImgStore } from '@/store/imgStore'
import { useAlbumStore } from '@/store/albumStore'
import { filesize } from 'filesize'
import { useRoute } from 'vue-router'
import { dater } from '@utils/dater'
import { Album } from '@type/types'
import { ref, watch } from 'vue'
import { editTitle } from '@utils/createAlbums'

const titleModel = ref('')
const route = useRoute()
const albumStore = useAlbumStore('mainId')
const imgStore = useImgStore('mainId')

const props = defineProps<{
  index: number
  album: Album
}>()

watch(
  () => props.album.title,
  () => {
    titleModel.value = props.album.title ?? ''
  },
  { immediate: true }
)
</script>

<style scoped>
.card-pair {
  display: flex;
  flex-direction: row;
}

@container image-col (aspect-ratio < 1) {
  .card-pair {
    flex-direction: column;
  }
}

.square {
  aspect-ratio: 1 / 1;
  inline-size: min(500px, max(min(100cqh, 50cqw), min(100cqw, 50cqh)));
  max-inline-size: 500px;
  max-block-size: 500px;
}

@supports not (container-type: size) {
  .square {
    inline-size: min(500px, 50vmin);
  }
}

.v-text-field :deep(input) {
  font-size: 2.125rem;
  font-weight: 400;
  line-height: 1.175;
  letter-spacing: 0.0073529412em;
}
</style>
