<script setup lang="ts">
import type { FacetValueInfo } from '@type/types'

const props = defineProps<{
  valueLabel: string
  items: readonly FacetValueInfo[]
  emptyText: string
}>()

const emit = defineEmits<{
  select: [value: string]
}>()

function selectValue(value: string): void {
  emit('select', value)
}
</script>

<template>
  <v-table hover>
    <thead>
      <tr>
        <th scope="col">{{ props.valueLabel }}</th>
        <th scope="col">Number of items</th>
      </tr>
    </thead>
    <tbody>
      <tr v-if="props.items.length === 0">
        <td colspan="2" class="text-medium-emphasis">{{ props.emptyText }}</td>
      </tr>
      <tr v-for="item in props.items" :key="item.value">
        <td>
          <v-btn
            slim
            class="text-body-small"
            variant="tonal"
            @click="selectValue(item.value)"
          >
            {{ item.value }}
          </v-btn>
        </td>
        <td>{{ item.count }}</td>
      </tr>
    </tbody>
  </v-table>
</template>
