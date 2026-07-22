<script setup lang="ts">
withDefaults(
  defineProps<{
    history: readonly string[]
    showEmpty?: boolean
  }>(),
  {
    showEmpty: false
  }
)

const emit = defineEmits<{
  select: [query: string]
  remove: [index: number]
  clear: []
}>()
</script>

<template>
  <v-list density="compact">
    <v-list-item
      v-if="history.length > 0"
      title="Recent Searches"
      density="compact"
      class="text-medium-emphasis"
    >
      <template #append>
        <v-btn
          text="Clear all"
          prepend-icon="mdi-delete-sweep-outline"
          size="small"
          variant="text"
          color="primary"
          aria-label="Clear all recent searches"
          @mousedown.stop.prevent
          @click.stop="emit('clear')"
        />
      </template>
    </v-list-item>
    <v-list-subheader v-else-if="showEmpty" title="No recent searches" />

    <v-list-item
      v-for="(item, index) in history"
      :key="item"
      :title="item"
      prepend-icon="mdi-magnify"
      @click="emit('select', item)"
    >
      <template #append>
        <v-btn
          icon="mdi-close"
          size="x-small"
          variant="text"
          :aria-label="`Remove recent search ${item}`"
          @mousedown.stop.prevent
          @click.stop="emit('remove', index)"
        />
      </template>
    </v-list-item>
  </v-list>
</template>
