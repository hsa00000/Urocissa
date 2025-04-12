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

<script setup lang="ts">
import { useUploadStore } from '@/store/uploadStore'
import { onMounted } from 'vue'
import { onUnmounted } from 'vue'
import { computed } from 'vue'
import { ref } from 'vue'
const uploadStore = useUploadStore('mainId')
const visible = ref(false)
const lastTarget = ref(null)

const classes = computed(() => {
  return {
    'vue-full-screen-file-drop--visible': visible.value
  }
})

function onDragEnter(e: DragEvent) {
  lastTarget.value = e.target as any

  const items = e.dataTransfer?.items
  if (items) {
    const hasValidType = Array.from(items).some(
      (item) => item.type.startsWith('image/') || item.type.startsWith('video/') || item.type === ''
    )

    if (hasValidType) {
      visible.value = true
    }
  }
}
function onDragLeave(e: any) {
  if (e.target === lastTarget.value) {
    visible.value = false
  }
}
function onDragOver(e: any) {
  e.preventDefault()
}
function onDrop(e: any) {
  e.preventDefault()
  visible.value = false
  const files: File[] = Array.from(e.dataTransfer.files)

  if (files.length !== 0) {
    uploadStore
      .fileUpload(files)
      .then((result) => {
        console.log(result)
      })
      .catch((error: unknown) => {
        console.error('Error occurred:', error)
      })
  }
}

onMounted(() => {
  window.addEventListener('dragenter', onDragEnter)
  window.addEventListener('dragleave', onDragLeave)
  window.addEventListener('dragover', onDragOver)
  window.addEventListener('drop', onDrop)
})

onUnmounted(() => {
  window.removeEventListener('dragenter', onDragEnter)
  window.removeEventListener('dragleave', onDragLeave)
  window.removeEventListener('dragover', onDragOver)
  window.removeEventListener('drop', onDrop)
})
</script>

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
