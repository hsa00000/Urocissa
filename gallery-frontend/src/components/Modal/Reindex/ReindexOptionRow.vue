<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  modelValue: boolean
  title: string
  description: string
  applicability: string
  danger?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const subtitle = computed(() => `${props.description} · ${props.applicability}`)
</script>

<template>
  <v-list-item
    link
    slim
    :active="danger === true && modelValue"
    :color="danger === true ? 'error' : undefined"
    :variant="danger === true && modelValue ? 'tonal' : 'text'"
    @click="emit('update:modelValue', !modelValue)"
  >
    <template #prepend>
      <v-checkbox-btn
        :model-value="modelValue"
        :color="danger === true ? 'error' : 'primary'"
        :aria-label="title"
        @click.stop
        @update:model-value="emit('update:modelValue', Boolean($event))"
      />
    </template>

    <v-list-item-title class="text-wrap">{{ title }}</v-list-item-title>
    <v-list-item-subtitle class="text-wrap">{{ subtitle }}</v-list-item-subtitle>
  </v-list-item>
</template>
