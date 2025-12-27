<template>
  <v-dialog
    v-model="internalValue"
    :max-width="width"
    variant="flat"
    persistent
    theme="dark"
    scrollable
    :fullscreen="fullscreen"
    :transition="transition"
    :id="id"
  >
    <v-card rounded="lg" class="d-flex flex-column" color="#212121">
      <slot name="header">
        <v-toolbar color="transparent" density="compact">
          <v-toolbar-title>
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
        <v-card-actions>
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
  // 保持 contentClass 讓外部控制內部 padding，預設仍為 pa-4 (Vuetify 標準卡片內距)
  contentClass: { type: String, default: 'pa-4' },
  loading: { type: Boolean, default: false },
  hideClose: { type: Boolean, default: false },
  fullscreen: { type: Boolean, default: false },
  transition: { type: String, default: 'dialog-transition' },
  id: { type: String, default: undefined }
})

const emit = defineEmits(['update:modelValue'])

const internalValue = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
})
</script>

<style scoped>
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
