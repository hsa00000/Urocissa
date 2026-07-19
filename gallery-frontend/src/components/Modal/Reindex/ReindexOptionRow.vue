<template>
  <v-list-item
    class="reindex-option rounded-lg px-3 py-2"
    :class="{ 'reindex-option--danger': danger && modelValue }"
    @click="emit('update:modelValue', !modelValue)"
  >
    <template #prepend>
      <v-checkbox-btn
        :model-value="modelValue"
        :color="danger ? 'error' : 'primary'"
        :aria-label="title"
        @click.stop
        @update:model-value="emit('update:modelValue', Boolean($event))"
      />
    </template>

    <v-list-item-title class="font-weight-medium">{{ title }}</v-list-item-title>
    <v-list-item-subtitle class="option-description mt-1">
      {{ description }}
    </v-list-item-subtitle>

    <template #append>
      <v-chip size="x-small" variant="tonal" :color="danger ? 'error' : undefined">
        {{ applicability }}
      </v-chip>
    </template>
  </v-list-item>
</template>

<script setup lang="ts">
defineProps<{
  modelValue: boolean
  title: string
  description: string
  applicability: string
  danger?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()
</script>

<style scoped>
.reindex-option {
  border: 1px solid rgba(255, 255, 255, 0.12);
  cursor: pointer;
}

.reindex-option:hover {
  background: rgba(255, 255, 255, 0.04);
}

.reindex-option--danger {
  border-color: rgba(var(--v-theme-error), 0.65);
  background: rgba(var(--v-theme-error), 0.08);
}

.option-description {
  white-space: normal;
  line-height: 1.35;
  opacity: 0.76;
}
</style>
