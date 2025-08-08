<template>
  <OverlayModal
    v-model="open"
    id="view-page"
    :close-on-back="true"
    :persistent="false"
    :transition="false"
    overlay-class="d-flex"
  >
    <Home isolation-id="tempId" :filter-string="filterString" :search-string="null">
      <template #reading-bar>
        <SelectionBar :album="album" />
      </template>
    </Home>
  </OverlayModal>
</template>
<script setup lang="ts">
import { Album } from '@type/types'
import Home from './Home.vue'
import SelectionBar from '@/components/NavBar/SelectionBar.vue'
import OverlayModal from '@/components/Modal/OverlayModal.vue'
import { ref, watch } from 'vue'

const props = defineProps<{
  album: Album
  modelValue: boolean
}>()

const emit = defineEmits<(e: 'update:modelValue', v: boolean) => void>()

const open = ref(props.modelValue)
watch(
  () => props.modelValue,
  v => (open.value = v)
)
watch(open, v => {
  emit('update:modelValue', v)
})

const filterString = `and(not(type:"album"), not(tag:"_trashed"), not(album:"${props.album.id}"))`
</script>
