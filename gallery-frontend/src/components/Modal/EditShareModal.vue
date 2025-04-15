<template>
  <v-dialog
    v-model="modalStore.showEditShareModal"
    id="share-modal"
    variant="flat"
    persistent
    rounded
  >
    <v-card
      class="h-100 mx-auto w-100"
      max-width="400"
      variant="elevated"
      retain-focus
      rounded="xl"
    >
      <v-toolbar color="transparent">
        <v-toolbar-title class="text-h5" text=" Share"></v-toolbar-title>
        <template #append>
          <v-btn icon="mdi-close" @click="modalStore.showEditShareModal = false"></v-btn>
        </template>
      </v-toolbar>
      <v-divider></v-divider>

      <v-list class="px-6" density="compact">
        <v-list-item>
          <v-textarea
            v-model="description"
            label="Description of this link"
            hide-details="auto"
            :style="{
              paddingBottom: 0
            }"
          ></v-textarea>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="requirePassword"
                color="primary"
                :label="`Require password`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <v-text-field
            v-model="password"
            label="Password"
            hide-details="auto"
            :disabled="!requirePassword"
            :style="{
              paddingBottom: 0
            }"
          ></v-text-field>
        </v-list-item>
        <v-list-item density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="allowDownload"
                color="primary"
                :label="`Allow public user to download`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="allowUpload"
                color="primary"
                :label="`Allow public user to upload`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="showMetadata"
                color="primary"
                :label="`Show metadata`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <template #prepend>
            <v-list-item-action start>
              <v-switch
                v-model="willExpire"
                color="primary"
                :label="`Expire after`"
                hide-details
              ></v-switch>
            </v-list-item-action>
          </template>
        </v-list-item>
        <v-list-item v-if="false" density="compact" slim>
          <v-select
            v-model="selectedDuration"
            :items="durations"
            label="Select a duration"
            item-title="label"
            item-value="id"
            hide-details="auto"
            :disabled="!willExpire"
          />
        </v-list-item>
      </v-list>
      <v-card-actions>
        <v-spacer></v-spacer>
        <v-btn
          color="teal-accent-4"
          variant="outlined"
          class="ma-2 button button-submit"
          @click="modalStore.showEditShareModal = false"
        >
          Cancel
        </v-btn>
        <v-btn
          color="teal-accent-4"
          variant="outlined"
          class="ma-2 button button-submit"
          type="submit"
        >
          Submit
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { useModalStore } from '@/store/modalStore'
import { ref, watchEffect } from 'vue'
import { EditShareData } from '@/type/types'

const props = defineProps<{
  editShareData: EditShareData
}>()

const modalStore = useModalStore('mainId')
const description = ref(props.editShareData.share.description)
const requirePassword = ref(false)
const password = ref('')
const willExpire = ref(false)
const allowUpload = ref(false)
const allowDownload = ref(true)
const showMetadata = ref(true)
const selectedDuration = ref<number | null>(null)

const durations = [
  { label: '30 minutes later', id: 30 },
  { label: '1 hour later', id: 60 },
  { label: '6 hours later', id: 360 },
  { label: '1 day later', id: 1440 },
  { label: '7 days later', id: 10080 },
  { label: '30 days later', id: 43200 },
  { label: '3 months later', id: 129600 },
  { label: '1 year later', id: 525600 }
]

watchEffect(() => {
  console.log('props.share.url is', props.editShareData.share.url)
})
</script>
