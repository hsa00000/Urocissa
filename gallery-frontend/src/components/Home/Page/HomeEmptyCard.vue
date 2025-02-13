<template>
  <v-container class="d-flex align-center justify-center" fluid>
    <v-row v-if="!isSearching" justify="center">
      <v-col
        v-if="route.meta.isReadPage && !collectionStore.editModeOn"
        class="w-100"
        cols="12"
        md="6"
        lg="4"
      >
        <v-hover v-slot="{ isHovering, props: hoverProps }">
          <v-card
            class="pa-4 text-center mx-auto"
            :class="{ 'hover-cursor': hasHoveringEffect }"
            :style="{
              border: isHovering ? '2px solid #BDBDBD' : '2px solid transparent'
            }"
            :elevation="isHovering ? 12 : 2"
            rounded="lg"
            width="100%"
            v-bind="hoverProps"
            @click="uploadStore.triggerFileInput()"
          >
            <v-icon class="mb-5" color="grey" size="100"> mdi-cloud-upload </v-icon>
            <v-card-item>
              <v-card-subtitle> Upload new photos. </v-card-subtitle>
            </v-card-item>
          </v-card>
        </v-hover>
      </v-col>
      <v-col class="w-100" cols="12" md="6" lg="4">
        <v-hover v-slot="{ isHovering, props: hoverProps }">
          <v-card
            class="pa-4 text-center mx-auto"
            :class="{ 'hover-cursor': hasHoveringEffect }"
            :style="{
              border:
                hasHoveringEffect && isHovering ? '2px solid #BDBDBD' : '2px solid transparent'
            }"
            :elevation="hasHoveringEffect && isHovering ? 12 : 2"
            rounded="lg"
            width="100%"
            v-bind="hasHoveringEffect ? hoverProps : props"
            @click="hasHoveringEffect ? clickEmptyCard() : undefined"
          >
            <v-icon class="mb-5" color="grey" size="100"> mdi-image-plus </v-icon>
            <v-card-item>
              <v-card-subtitle>
                {{ computedMessage }}
              </v-card-subtitle>
            </v-card-item>
          </v-card>
        </v-hover>
      </v-col>
    </v-row>
    <v-row justify="center" v-else>
      <v-col class="w-100" cols="12" md="6" lg="4">
        <v-hover v-slot="{ isHovering }">
          <v-card
            class="pa-4 text-center mx-auto"
            :class="{ 'hover-cursor': hasHoveringEffect }"
            :style="{
              border:
                hasHoveringEffect && isHovering ? '2px solid #BDBDBD' : '2px solid transparent'
            }"
            :elevation="hasHoveringEffect && isHovering ? 12 : 2"
            rounded="lg"
            width="100%"
            v-bind="props"
            @click="undefined"
          >
            <v-icon class="mb-5" color="grey" size="100"> mdi-book-remove-multiple </v-icon>
            <v-card-item>
              <v-card-subtitle>
                {{ 'Result not found.' }}
              </v-card-subtitle>
            </v-card-item>
          </v-card>
        </v-hover>
      </v-col>
    </v-row>
  </v-container>
</template>
<script setup lang="ts">
import { createEmptyAlbum } from '@/script/common/createAlbums'
import { IsolationId } from '@/script/common/types'
import { useCollectionStore } from '@/store/collectionStore'
import { useModalStore } from '@/store/modalStore'
import { useUploadStore } from '@/store/uploadStore'
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const props = defineProps<{
  isolationId: IsolationId
}>()

const route = useRoute()
const router = useRouter()

const uploadStore = useUploadStore(props.isolationId)
const collectionStore = useCollectionStore(props.isolationId)
const modalStore = useModalStore('mainId')

const isSearching = computed(() => {
  return route.query.search !== undefined && route.query.search?.toString() !== ''
})

const hasHoveringEffect = computed(() => {
  const path = route.path
  if (path.startsWith('/favorite')) {
    return false
  } else if (path.startsWith('/archived')) {
    return false
  } else if (path.startsWith('/trashed')) {
    return false
  } else if (route.meta.isReadPage) {
    return collectionStore.editModeOn ? false : true // Inside the component for adding photos
  } else if (path.startsWith('/albums')) {
    return true
  } else if (path.startsWith('/all')) {
    return true
  } else if (path.startsWith('/view')) {
    return false
  } else if (path.startsWith('/home')) {
    return true
  } else {
    return true
  }
})

const computedMessage = computed(() => {
  const path = route.path
  if (path.startsWith('/favorite')) {
    return 'Add your favorite photos and videos here!'
  } else if (path.startsWith('/archived')) {
    return 'Archived photos won’t appear on the home page.'
  } else if (path.startsWith('/trashed')) {
    return 'Deleted photos and videos appear here.'
  } else if (path.startsWith('/all')) {
    return 'Upload some photos here!'
  } else if (route.meta.isReadPage) {
    return collectionStore.editModeOn
      ? 'All photos are already added!' // Inside the component for adding photos
      : 'Select from existing photos.'
  } else if (path.startsWith('/home')) {
    return 'Upload some photos here!'
  } else if (path.startsWith('/albums')) {
    return 'Create some albums here!'
  } else {
    return 'Upload some photos here!'
  }
})

const clickEmptyCard = async () => {
  const path = route.path

  if (route.meta.isReadPage) {
    modalStore.showHomeTempModal = true
  } else if (path.startsWith('/albums')) {
    await createEmptyAlbum(props.isolationId, router)
  } else {
    uploadStore.triggerFileInput()
  }
}
</script>
<style scoped>
.hover-cursor {
  cursor: pointer !important;
}
</style>
