<template>
  <v-col
    id="abstractData-col"
    v-if="abstractData"
    class="h-100 abstractData-css"
    cols="auto"
    :style="{ backgroundColor: 'white' }"
  >
    <v-row no-gutters class="position-relative">
      <v-toolbar
        color="white"
        :style="{
          backgroundColor: '#212121'
        }"
      >
        <!-- Icon button with increased size -->
        <v-btn icon @click="toggleInfo">
          <v-icon>mdi-close</v-icon>
        </v-btn>
        <v-toolbar-title class="text-h5">Info</v-toolbar-title>
      </v-toolbar>
      <v-col v-if="abstractData.database" class="h-100 w-100" cols="auto">
        <v-list bg-color="white" class="pa-0" height="100%" lines="two">
          <ItemSize :database="abstractData.database" />
          <ItemPath v-if="showMetadata" :database="abstractData.database" />
          <ItemDate :database="abstractData.database" />
          <ItemExif
            v-if="
              abstractData.database.exif_vec.Make !== undefined ||
              abstractData.database.exif_vec.Model !== undefined
            "
            :database="abstractData.database"
          />
          <v-divider></v-divider>
          <ItemTag
            v-if="showMetadata"
            :isolation-id="props.isolationId"
            :index="props.index"
            :tags="abstractData.database.tag"
          />
          <ItemAlbum
            v-if="route.meta.baseName !== 'share'"
            :isolation-id="props.isolationId"
            :index="props.index"
            :albums="abstractData.database.album"
          />
        </v-list>
      </v-col>
      <v-col v-if="abstractData.album" class="h-100 w-100" cols="auto">
        <v-list bg-color="white" class="pa-0" height="100%" lines="two">
          <ItemTitle :title="abstractData.album.title" />
          <ItemCount :album="abstractData.album" />

          <v-divider></v-divider>
          <ItemTag
            :isolation-id="props.isolationId"
            :index="props.index"
            :tags="abstractData.album.tag"
          />
        </v-list>
      </v-col>
    </v-row>
  </v-col>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import { useInfoStore } from '@/store/infoStore'

import { AbstractData, IsolationId } from '@type/types'

import ItemExif from './ItemExif.vue'
import ItemSize from './ItemSize.vue'
import ItemPath from './ItemPath.vue'
import ItemDate from './ItemDate.vue'
import ItemTag from './ItemTag.vue'
import ItemAlbum from './ItemAlbum.vue'
import ItemTitle from './ItemTitle.vue'
import ItemCount from './ItemCount.vue'
import { useRoute } from 'vue-router'
import { useShareStore } from '@/store/shareStore'

const route = useRoute()

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: AbstractData
}>()

const showMetadata = computed(() => {
  return route.meta.baseName !== 'share' || shareStore.resolvedShare?.share.showMetadata
})
const infoStore = useInfoStore('mainId')
const shareStore = useShareStore('mainId')

function toggleInfo() {
  infoStore.showInfo = !infoStore.showInfo
}

watch(
  () => props.hash,
  () => {
    console.log(props.abstractData)
  }
)
</script>
<style scoped>
.abstractData-css {
  width: 360px;
}

@media (max-width: 720px) {
  .abstractData-css {
    width: 100%;
  }
}
</style>
