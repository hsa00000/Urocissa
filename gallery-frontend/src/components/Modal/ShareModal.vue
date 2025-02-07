<template>
  <v-dialog v-model="modalStore.showShareModal" id="share-modal" variant="flat" persistent style="">
    <v-card class="h-100 mx-auto w-100" max-width="400" variant="elevated" retain-focus>
      <v-toolbar color="transparent">
        <v-toolbar-title class="text-h5" text=" Share"></v-toolbar-title>
        <template #append>
          <v-btn icon="mdi-dots-vertical"></v-btn>
        </template>
      </v-toolbar>
      <v-divider></v-divider>
      <v-list class="px-6" density="compact">
        <v-list-item density="compact" slim>
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
        <v-list-item density="compact" slim>
          <v-text-field
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
        <v-list-item density="compact" slim>
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
        <v-list-item density="compact" slim>
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
        <v-list-item density="compact" slim>
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
        <v-list-item density="compact" slim>
          <v-select
            hide-details="auto"
            label="Select"
            :items="['California', 'Colorado', 'Florida', 'Georgia', 'Texas', 'Wyoming']"
          ></v-select>
        </v-list-item>
      </v-list>
      <v-card-actions>
        <v-spacer></v-spacer>
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
import { ref } from 'vue'
const modalStore = useModalStore('mainId')
const requirePassword = ref(false)
const willExpire = ref(false)
const allowUpload = ref(false)
const allowDownload = ref(true)
const showMetadata = ref(false)
</script>
