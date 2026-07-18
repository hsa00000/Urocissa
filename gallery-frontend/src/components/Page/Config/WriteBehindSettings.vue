<script setup lang="ts">
import type { WriteBehindConfig } from '@/api/config'

const writeBehind = defineModel<WriteBehindConfig>({ required: true })

const intervalRules = [
  (value: number) => (value >= 100 && value <= 60_000) || 'Use 100–60,000 ms'
]
const softRules = [
  (value: number) => value > 0 || 'Must be positive',
  (value: number) => value < writeBehind.value.hardLimitMiB || 'Must be below hard limit'
]
const hardRules = [
  (value: number) => value <= 256 || 'Maximum is 256 MiB',
  (value: number) => value > writeBehind.value.softLimitMiB || 'Must exceed soft limit'
]
</script>

<template>
  <v-list-item
    title="RAM-first write-behind"
    subtitle="Edits are acknowledged from RAM. A process crash or power loss can discard pending edits."
  />
  <v-list-item>
    <div class="write-behind-grid w-100">
      <v-number-input
        v-model="writeBehind.flushIntervalMs"
        label="Flush interval (ms)"
        :min="100"
        :max="60000"
        :rules="intervalRules"
        variant="outlined"
        density="compact"
      />
      <v-number-input
        v-model="writeBehind.softLimitMiB"
        label="Soft limit (MiB)"
        :min="1"
        :max="255"
        :rules="softRules"
        variant="outlined"
        density="compact"
      />
      <v-number-input
        v-model="writeBehind.hardLimitMiB"
        label="Hard limit (MiB)"
        :min="2"
        :max="256"
        :rules="hardRules"
        variant="outlined"
        density="compact"
      />
    </div>
  </v-list-item>
</template>

<style scoped>
.write-behind-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  padding-block: 8px;
}
</style>
