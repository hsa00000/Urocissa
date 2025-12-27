<template>
  <v-card
    class="mx-auto position-fixed"
    append-icon=""
    :title="`${uploadStore.status}`"
    :subtitle="`${humanizeDuration(uploadStore.remainingTime * 1000, {
      units: ['h', 'm', 's'],
      largest: 1,
      round: true
    })} remaining`"
    variant="elevated"
    id="upload-vcard"
    retain-focus
    :style="{
      bottom: '50px',
      left: '50px',
      zIndex: 50000
    }"
  >
    <template #prepend>
      <div class="progress-container">
        <Transition name="fade-scale" mode="out-in">
          <v-progress-circular
            key="progress-{{uploadStore.status}}"
            color="primary"
            :model-value="circularValue"
            :indeterminate="uploadStore.status === 'Processing'"
            class="ma-4"
          >
            <Transition name="fade-scale" mode="out-in">
              <v-icon :key="uploadStore.status" :icon="circularIcon" />
            </Transition>
          </v-progress-circular>
        </Transition>
      </div>
    </template>
    <template #append>
      <v-btn
        variant="outlined"
        class="ma-4"
        style="width: 100px"
        @click="modalStore.showUploadModal = false"
        v-if="uploadStore.status === 'Completed' || uploadStore.status === 'Canceled'"
      >
        Close
      </v-btn>
      <v-btn
        v-else
        variant="outlined"
        class="ma-4"
        style="width: 100px"
        @click="uploadStore.cancelUpload()"
      >
        Cancel
      </v-btn>
    </template>
  </v-card>
</template>
<script setup lang="ts">
/**
 * This modal is used for displaying upload information.
 */
import { useModalStore } from '@/store/modalStore'
import { useUploadStore } from '@/store/uploadStore'
import humanizeDuration from 'humanize-duration'
import { computed } from 'vue'
const uploadStore = useUploadStore('mainId')
const modalStore = useModalStore('mainId')

const circularValue = computed(() => {
  if (uploadStore.status === 'Uploading') {
    return uploadStore.percentComplete
  } else if (uploadStore.status === 'Completed') {
    return 0
  } else {
    return undefined
  }
})

const circularIcon = computed(() => {
  if (uploadStore.status === 'Completed') {
    return 'mdi-cloud-check-variant'
  } else {
    return 'mdi-cloud-upload'
  }
})
</script>
<style scoped>
.fade-scale-enter-active,
.fade-scale-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}
.fade-scale-enter-from,
.fade-scale-leave-to {
  opacity: 0;
  transform: scale(0.8);
}
.fade-scale-enter-to,
.fade-scale-leave-from {
  opacity: 1;
  transform: scale(1);
}
</style>
