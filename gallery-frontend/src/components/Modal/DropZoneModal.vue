<!--
MIT License

Copyright (c) 2017 Christian Catalan

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
-->

<!--
// https://www.npmjs.com/package/vue-full-screen-file-drop
// https://github.com/crcatala/vue-full-screen-file-drop/tree/master
// Based on a Vue 3 compatible version by @d1y in GitHub Issue #3
// https://github.com/crcatala/vue-full-screen-file-drop/issues/3
-->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, shallowRef } from 'vue'
import { useRoute } from 'vue-router'
import { handleClipboardImagePaste } from '@/script/utils/clipboardImageFiles'
import {
  isUploadIngressAllowed,
  resolveUploadIngress,
  type UploadIngressContext
} from '@/script/utils/uploadIngress'
import { useAlbumStore } from '@/store/albumStore'
import { useMessageStore } from '@/store/messageStore'
import { useShareStore } from '@/store/shareStore'
import { useUploadStore } from '@/store/uploadStore'

const uploadStore = useUploadStore('mainId')
const shareStore = useShareStore('mainId')
const messageStore = useMessageStore('mainId')
const albumStore = useAlbumStore('mainId')
const visible = shallowRef(false)
const lastTarget = shallowRef<EventTarget | null>(null)
const route = useRoute()

const classes = computed(() => ({
  'vue-full-screen-file-drop--visible': visible.value
}))

function isUploadAllowed(event: DragEvent): boolean {
  if (!isUploadIngressAllowed(route.meta.level)) return false

  const items = event.dataTransfer?.items
  if (!items) return false

  return Array.from(items).some(
    (item) => item.type.startsWith('image/') || item.type.startsWith('video/') || item.type === ''
  )
}

function onDragEnter(event: DragEvent): void {
  if (!isUploadAllowed(event)) return
  lastTarget.value = event.target
  visible.value = true
}

function onDragLeave(event: DragEvent): void {
  if (event.target === lastTarget.value) {
    visible.value = false
  }
}

function onDragOver(event: DragEvent): void {
  event.preventDefault()
}

function getUploadIngressContext(): UploadIngressContext {
  const albumId = shareStore.albumId ?? undefined
  const shareId = shareStore.shareId ?? undefined
  const isSharedAlbum = typeof albumId === 'string' && typeof shareId === 'string'
  const hashParam = route.params.hash
  const isLevel3RouteWithHash = route.meta.level === 3 && typeof hashParam === 'string'

  return {
    routeLevel: route.meta.level,
    isUploadPage: route.name === 'upload',
    routeAlbum: isLevel3RouteWithHash
      ? { id: hashParam, name: albumStore.albums.get(hashParam)?.displayName }
      : undefined,
    sharedUpload: isSharedAlbum
      ? {
          albumId,
          shareId,
          password: shareStore.password,
          albumTitle: shareStore.resolvedShare?.albumTitle,
          uploadAllowed: shareStore.resolvedShare?.share.showUpload === true
        }
      : undefined,
    presignedAlbums: uploadStore.presignAlbumIds.flatMap((presignedAlbumId) => {
      const album = albumStore.albums.get(presignedAlbumId)
      return album === undefined ? [] : [{ id: album.albumId, name: album.displayName }]
    }),
    presignedTags: uploadStore.presignTags
  }
}

function enqueueFilesForCurrentRoute(files: readonly File[]): void {
  if (files.length === 0) return

  const resolution = resolveUploadIngress(getUploadIngressContext())
  if (resolution.kind === 'blocked-route') return
  if (resolution.kind === 'blocked-share') {
    messageStore.error('Public uploads are not allowed for this album share setting.')
    return
  }

  uploadStore.enqueueFiles(files, resolution.target, {
    showSummaryPanel: resolution.showSummaryPanel
  })
}

function onDrop(event: DragEvent): void {
  event.preventDefault()
  if (!isUploadAllowed(event)) return

  visible.value = false
  enqueueFilesForCurrentRoute(Array.from(event.dataTransfer?.files ?? []))
}

function onPaste(event: ClipboardEvent): void {
  if (!isUploadIngressAllowed(route.meta.level)) return
  handleClipboardImagePaste(event, enqueueFilesForCurrentRoute)
}

onMounted(() => {
  window.addEventListener('dragenter', onDragEnter)
  window.addEventListener('dragleave', onDragLeave)
  window.addEventListener('dragover', onDragOver)
  window.addEventListener('drop', onDrop)
  window.addEventListener('paste', onPaste, true)
})

onUnmounted(() => {
  window.removeEventListener('dragenter', onDragEnter)
  window.removeEventListener('dragleave', onDragLeave)
  window.removeEventListener('dragover', onDragOver)
  window.removeEventListener('drop', onDrop)
  window.removeEventListener('paste', onPaste, true)
})
</script>

<template>
  <div class="vue-full-screen-file-drop" :class="classes">
    <slot>
      <div class="vue-full-screen-file-drop__content">
        <div class="d-flex flex-column align-center">
          <v-icon size="128" icon="mdi-cloud-upload" class="mb-5" />
          <div class="mt-3 text-center" style="font-size: 2rem; font-weight: bold">
            Drag and drop files here
          </div>
        </div>
      </div>
    </slot>
  </div>
</template>

<style lang="css">
.vue-full-screen-file-drop {
  position: fixed;
  top: 0;
  left: 0;
  z-index: 10000;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.4);
  visibility: hidden;
  opacity: 0;
  transition: visibility 200ms, opacity 200ms;
}

.vue-full-screen-file-drop--visible {
  opacity: 1;
  visibility: visible;
}

.vue-full-screen-file-drop__content {
  display: flex;
  justify-content: center;
  align-items: center;
  width: 100%;
  height: 100%;
  color: #fff;
  font-size: 4em;
}

.vue-full-screen-file-drop__content:before {
  content: '';
  bottom: 60px;
  left: 60px;
  position: absolute;
  right: 60px;
  top: 60px;
}
</style>
