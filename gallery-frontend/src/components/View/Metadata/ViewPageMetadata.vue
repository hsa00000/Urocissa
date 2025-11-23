<template>
  <div
    id="abstractData-col"
    v-if="abstractData"
    class="h-100 flex-grow-0 flex-shrink-0 bg-surface"
    :style="{
      width: constStore.showInfo ? '360px' : '0',
      zIndex: 1
    }"
  >
    <div class="position-relative">
      <div>
        <v-toolbar class="bg-surface">
          <v-btn icon @click="toggleInfo">
            <v-icon>mdi-close</v-icon>
          </v-btn>
          <v-toolbar-title class="text-h5">Info</v-toolbar-title>
        </v-toolbar>
      </div>
      <v-card-item>
        <v-textarea
          v-model="userDefinedDescriptionModel"
          variant="underlined"
          rows="1"
          auto-grow
          @blur="
            editUserDefinedDescription(
              props.abstractData,
              userDefinedDescriptionModel,
              props.index,
              props.isolationId
            )
          "
          :placeholder="userDefinedDescriptionModel === '' ? 'Add description' : undefined"
        />
      </v-card-item>
      <div v-if="abstractData.database" class="h-100 w-100">
        <v-list class="pa-0" height="100%" lines="two">
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
            :tags="abstractData.database.tags"
          />
          <ItemAlbum
            v-if="route.meta.baseName !== 'share'"
            :isolation-id="props.isolationId"
            :index="props.index"
            :albums="abstractData.database.album"
          />
        </v-list>
      </div>
      <div v-if="abstractData.album" class="h-100 w-100">
        <v-list class="pa-0" height="100%" lines="two">
          <ItemTitle :title="abstractData.album.title" />
          <ItemCount :album="abstractData.album" />
          <v-divider></v-divider>
          <ItemTag
            :isolation-id="props.isolationId"
            :index="props.index"
            :tags="abstractData.album.tag"
          />
        </v-list>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, watch, ref } from 'vue'
import { useConstStore } from '@/store/constStore'

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
import { editUserDefinedDescription } from '@utils/editDescription'

const route = useRoute()

const userDefinedDescriptionModel = ref('')

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: AbstractData
}>()

const showMetadata = computed(() => {
  return route.meta.baseName !== 'share' || shareStore.resolvedShare?.share.showMetadata
})
const constStore = useConstStore('mainId')
const shareStore = useShareStore('mainId')

function toggleInfo() {
  void constStore.updateShowInfo(!constStore.showInfo)
}

function getUserDefinedDescription(abstractData: AbstractData): string {
  if (abstractData.database) {
    return abstractData.database.exif_vec._user_defined_description ?? ''
  } else if (abstractData.album) {
    return abstractData.album.user_defined_metadata._user_defined_description?.[0] ?? ''
  }
  return ''
}

watch(
  () => props.hash,
  () => {
    userDefinedDescriptionModel.value = getUserDefinedDescription(props.abstractData)
  },
  { immediate: true }
)
</script>

<style scoped>
@media (width <= 720px) {
  /* On small screens, make the info pane full width.
     Use !important to override the inline :style binding for width. */
  #abstractData-col {
    width: 100% !important;
  }
}
</style>
