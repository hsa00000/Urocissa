<template>
  <v-dialog
    v-model="internalValue"
    :max-width="width"
    variant="flat"
    persistent
    theme="dark"
    scrollable
    :fullscreen="fullscreen"
    :id="id"
  >
    <v-card rounded="xl" class="d-flex flex-column" color="#212121">
      <slot name="header">
        <v-toolbar color="transparent" density="compact" class="px-2 pt-1">
          <v-toolbar-title class="text-h6 font-weight-bold ml-2">
            {{ title }}
          </v-toolbar-title>
          <template #append>
            <v-btn
              v-if="!hideClose"
              icon="mdi-close"
              variant="text"
              density="comfortable"
              :disabled="loading"
              @click="internalValue = false"
            ></v-btn>
          </template>
        </v-toolbar>
      </slot>

      <v-progress-linear
        v-if="loading"
        indeterminate
        color="primary"
        height="2"
      ></v-progress-linear>
      <v-divider v-else class="border-opacity-25"></v-divider>

      <v-card-text :class="['custom-scrollbar', contentClass]">
        <slot></slot>
      </v-card-text>

      <template v-if="$slots.actions">
        <v-card-actions class="pa-4 pt-2">
          <slot name="actions"></slot>
        </v-card-actions>
      </template>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps({
  modelValue: { type: Boolean, required: true },
  title: { type: String, default: '' },
  width: { type: [String, Number], default: 450 },
  // 允許外部覆寫 padding，例如 UploadModal 可以設為 'pa-0'
  contentClass: { type: String, default: 'pa-4' },
  // 統一的 Loading 狀態控制
  loading: { type: Boolean, default: false },
  // 是否隱藏右上角關閉按鈕
  hideClose: { type: Boolean, default: false },
  // 是否全螢幕
  fullscreen: { type: Boolean, default: false },
  // 傳遞 ID 以便 CSS 定位 (如 original code 中的 id="edit-tag-overlay")
  id: { type: String, default: undefined }
})

const emit = defineEmits(['update:modelValue'])

const internalValue = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
})
</script>

<style scoped>
/* 這是 BaseModal 唯一保留的 Style，確保所有 Modal 滾動條一致 */
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: rgba(255, 255, 255, 0.2);
  border-radius: 4px;
}
</style>
