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
            :class="{ 'hover-cursor': computedHoverAndMessage.hasHoverEffect }"
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
            :class="{ 'hover-cursor': computedHoverAndMessage.hasHoverEffect }"
            :style="{
              border:
                computedHoverAndMessage.hasHoverEffect && isHovering ? '2px solid #BDBDBD' : '2px solid transparent'
            }"
            :elevation="computedHoverAndMessage.hasHoverEffect && isHovering ? 12 : 2"
            rounded="lg"
            width="100%"
            v-bind="computedHoverAndMessage.hasHoverEffect ? hoverProps : props"
            @click="computedHoverAndMessage.hasHoverEffect ? clickEmptyCard() : undefined"
          >
            <v-icon class="mb-5" color="grey" size="100"> mdi-image-plus </v-icon>
            <v-card-item>
              <v-card-subtitle>
                {{ computedHoverAndMessage.message }}
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
            :class="{ 'hover-cursor': computedHoverAndMessage.hasHoverEffect }"
            :style="{
              border:
                computedHoverAndMessage.hasHoverEffect && isHovering
                  ? '2px solid #BDBDBD'
                  : '2px solid transparent'
            }"
            :elevation="computedHoverAndMessage.hasHoverEffect && isHovering ? 12 : 2"
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

interface HoverAndMessage {
  hasHoverEffect: boolean
  message: string
}

const computedHoverAndMessage = computed<HoverAndMessage>(() => {
  if (collectionStore.editModeOn) {
    return {
      hasHoverEffect: false,
      message: route.meta.isReadPage
        ? 'All photos are already added!' // Inside the component for adding photos
        : 'Select from existing photos.'
    }
  }

  switch (route.meta.baseName) {
    case 'home':
      return { hasHoverEffect: true, message: 'Upload some photos here!' }
    case 'all':
      return { hasHoverEffect: true, message: 'Upload some photos here!' }
    case 'favorite':
      return { hasHoverEffect: false, message: 'Add your favorite photos and videos here!' }
    case 'archived':
      return { hasHoverEffect: false, message: 'Archived photos won’t appear on the home page.' }
    case 'trashed':
      return { hasHoverEffect: false, message: 'Deleted photos and videos appear here.' }
    case 'albums':
      return { hasHoverEffect: true, message: 'Create some albums here!' }
    case 'videos':
      return { hasHoverEffect: false, message: 'Upload some videos here!' }
    case 'album':
      return { hasHoverEffect: true, message: 'Upload some photos here!' }
    case 'tags':
      return { hasHoverEffect: false, message: 'Organize your photos with tags!' }
    case 'login':
      return { hasHoverEffect: false, message: 'Sign in to access your photos.' }
    default:
      return { hasHoverEffect: false, message: 'Upload some photos here!' }
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
