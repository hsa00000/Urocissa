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
    <v-list-subheader v-if="history.length > 0">
      <div class="d-flex align-center w-100">
        <span>Recent Searches</span>
        <v-spacer />
        <v-btn
          prepend-icon="mdi-delete-sweep-outline"
          size="small"
          variant="text"
          aria-label="Clear all recent searches"
          @mousedown.stop.prevent
          @click.stop="emit('clear')"
        >
          Clear all
        </v-btn>
      </div>
    </v-list-subheader>
    <v-list-subheader v-else-if="showEmpty">No recent searches</v-list-subheader>

    <v-list-item
      v-for="(item, index) in history"
      :key="item"
      @click="emit('select', item)"
    >
      <template #prepend>
        <v-icon size="small">mdi-magnify</v-icon>
      </template>

      <v-list-item-title class="text-body-2">{{ item }}</v-list-item-title>

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
