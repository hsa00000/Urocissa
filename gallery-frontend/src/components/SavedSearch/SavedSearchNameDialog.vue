<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue'
import BaseModal from '@/components/Modal/BaseModal.vue'

const props = withDefaults(
  defineProps<{
    title: string
    initialName?: string
    existingNames?: readonly string[]
    loading?: boolean
  }>(),
  {
    initialName: '',
    existingNames: () => [],
    loading: false
  }
)

const isOpen = defineModel<boolean>({ required: true })
const emit = defineEmits<{
  submit: [name: string]
}>()

const name = shallowRef('')
function countCharacters(value: string | null | undefined): number {
  return Array.from(value ?? '').length
}

const normalizedExistingNames = computed(
  () => new Set(props.existingNames.map((item) => item.trim().toLowerCase()))
)
const normalizedName = computed(() => name.value.trim())
const nameCharacterCount = computed(() => countCharacters(normalizedName.value))
const nameError = computed<string | null>(() => {
  if (normalizedName.value === '') return 'Name is required.'
  if (nameCharacterCount.value > 80) return 'Name must be 80 characters or fewer.'
  if (normalizedExistingNames.value.has(normalizedName.value.toLowerCase())) {
    return 'This name is already in use.'
  }
  return null
})
const canSubmit = computed(() => !props.loading && nameError.value === null)

watch(
  isOpen,
  (open) => {
    if (open) name.value = props.initialName
  },
  { immediate: true }
)

function submit(): void {
  if (!canSubmit.value) return
  emit('submit', normalizedName.value)
}
</script>

<template>
  <BaseModal v-model="isOpen" :title="title" :loading="loading" :width="450">
    <v-form validate-on="input" @submit.prevent="submit">
      <v-text-field
        v-model="name"
        autofocus
        label="Name"
        variant="outlined"
        counter="80"
        :counter-value="countCharacters"
        :error-messages="nameError === null ? [] : [nameError]"
        :disabled="loading"
      />
    </v-form>

    <template #actions>
      <v-spacer />
      <v-btn variant="text" :disabled="loading" @click="isOpen = false">Cancel</v-btn>
      <v-btn
        prepend-icon="mdi-content-save-outline"
        color="primary"
        variant="flat"
        :loading="loading"
        :disabled="!canSubmit"
        @click="submit"
      >
        Save
      </v-btn>
    </template>
  </BaseModal>
</template>
