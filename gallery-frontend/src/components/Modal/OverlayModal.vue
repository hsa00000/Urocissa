<template>
  <v-overlay
    v-model="internal"
    :class="['d-flex', overlayClass]"
    :height="height"
    :width="width"
    :transition="transition"
    :close-on-back="closeOnBack"
    :persistent="persistent"
    :scrim="scrim"
    :z-index="zIndex"
    :contained="contained"
  >
    <slot />
  </v-overlay>
  
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    height?: string | number
    width?: string | number
    transition?: string | false
    closeOnBack?: boolean
    persistent?: boolean
    scrim?: boolean | string
  zIndex?: number | string
    contained?: boolean
    overlayClass?: string
  }>(),
  {
    height: '100%',
    width: '100%',
    transition: false,
    closeOnBack: true,
    persistent: false,
    scrim: true,
    overlayClass: '',
    zIndex: 2000
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'open' | 'close'): void
}>()

const internal = computed({
  get: () => props.modelValue,
  set: (v: boolean) => {
    emit('update:modelValue', v)
  }
})

watch(
  () => props.modelValue,
  v => {
    if (v) emit('open')
    else emit('close')
  },
  { immediate: false }
)
</script>
