<script setup lang="ts">
const internalValue = defineModel<boolean>({ required: true })

withDefaults(
  defineProps<{
    title?: string
    width?: string | number
    /** External control of inner padding. Default: 'pa-4' (Standard) */
    contentClass?: string
    loading?: boolean
    hideClose?: boolean
    fullscreen?: boolean
    transition?: string
    id?: string
  }>(),
  {
    title: '',
    width: 450,
    contentClass: 'pa-4',
    loading: false,
    hideClose: false,
    fullscreen: false,
    transition: 'dialog-transition',
    id: undefined
  }
)
</script>

<template>
  <v-dialog
    :id="id"
    v-model="internalValue"
    :max-width="width"
    persistent
    scrollable
    :fullscreen="fullscreen"
    :transition="transition"
  >
    <v-card color="surface" rounded="lg" class="d-flex flex-column">
      <slot name="header">
        <v-toolbar color="transparent" density="compact">
          <v-toolbar-title>
            {{ title }}
          </v-toolbar-title>

          <template #append>
            <v-btn
              v-if="!hideClose"
              icon="mdi-close"
              aria-label="Close dialog"
              variant="text"
              density="comfortable"
              :disabled="loading"
              @click="internalValue = false"
            />
          </template>
        </v-toolbar>
      </slot>

      <v-progress-linear v-if="loading" indeterminate color="primary" height="2" />
      <v-divider v-else class="border-opacity-25" />

      <v-card-text :class="['custom-scrollbar', contentClass]">
        <slot />
      </v-card-text>

      <template v-if="$slots.actions">
        <v-card-actions>
          <slot name="actions" />
        </v-card-actions>
      </template>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background-color: rgb(var(--v-theme-on-surface));
  opacity: var(--v-disabled-opacity);
  border-radius: 4px;
}
</style>
