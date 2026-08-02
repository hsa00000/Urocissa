<template>
  <div class="h-100 d-flex align-center justify-center">
    <div class="card-pair">
      <v-card
        class="square album-cover-card rounded-0"
        style="object-fit: cover; border: 8px solid #fff"
      >
        <img
          v-if="imgStore.imgOriginal.get(index)"
          id="album-img"
          :key="index"
          :src="imgStore.imgOriginal.get(index)"
          class="w-100 h-100"
          style="object-fit: cover"
        />
      </v-card>

      <v-card class="square album-info-card d-flex flex-column pa-4 rounded-0">
        <v-card-item>
          <v-text-field
            v-model="titleModel"
            variant="underlined"
            @blur="editTitle(props.album, titleModel, isolationId)"
            :placeholder="titleModel === '' ? 'Add Title' : undefined"
          />
        </v-card-item>

        <v-list class="album-meta-list">
          <v-list-item class="album-meta-item">
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
import type { GalleryAlbum, IsolationId } from '@type/types'
import { ref, watch } from 'vue'
import { editTitle } from '@utils/createAlbums'

const titleModel = ref('')
const route = useRoute()
const albumStore = useAlbumStore('mainId')
const props = defineProps<{
  index: number
  album: GalleryAlbum
  isolationId: IsolationId
}>()
const imgStore = useImgStore(props.isolationId)

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
  --album-square-size: min(500px, max(min(100cqh, 50cqw), min(100cqw, 50cqh)));
}

@container image-col (aspect-ratio < 1) {
  .card-pair {
    flex-direction: column;
    --album-square-size: min(500px, min(100cqw, 50cqh));
  }

  .album-info-card {
    padding: 12px !important;
  }

  .album-info-card :deep(.v-card-item) {
    padding: 0 0 8px;
    min-height: 0;
  }

  .album-info-card :deep(.v-card-actions) {
    padding: 8px 0 0;
  }

  .album-info-card .v-text-field :deep(input) {
    font-size: 1.75rem;
  }
}

.square {
  aspect-ratio: 1 / 1;
  inline-size: var(--album-square-size);
  block-size: var(--album-square-size);
  max-inline-size: 500px;
  max-block-size: 500px;
  box-sizing: border-box;
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

.album-meta-list {
  max-inline-size: 100%;
  overflow-x: hidden;
  --v-list-indent-size: 0px;
}

.album-meta-item :deep(.v-list-item),
.album-meta-item :deep(.v-list-item__content) {
  min-inline-size: 0;
}

.album-meta-item :deep(.v-list-item-title),
.album-meta-item :deep(.v-list-item-subtitle) {
  overflow-wrap: anywhere;
}
</style>
