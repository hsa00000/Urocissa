<template>
  <PageTemplate
    preset="card"
    width="narrow"
    card-class="config-page-surface bg-transparent overflow-y-auto w-100"
    :ready="initializedStore.initialized"
  >
    <template #content>
      <div class="config-stack">
        <div class="config-block">
          <v-row class="ma-0">
            <ChangePassword v-model:has-password="localSettings.hasPassword" />
          </v-row>
        </div>

        <div class="config-block">
          <v-row class="ma-0">
            <StorageAndSync v-model:sync-paths="localSettings.syncPaths" />
          </v-row>
        </div>

        <div class="config-block">
          <v-row class="ma-0">
            <FolderImport />
          </v-row>
        </div>

        <div class="config-block">
          <v-row class="ma-0">
            <AdvancedConfig
              v-model:auth-key="localSettings.authKey"
              v-model:has-auth-key="localSettings.hasAuthKey"
              v-model:discord-hook-url="localSettings.discordHookUrl"
              v-model:read-only-mode="localSettings.readOnlyMode"
              v-model:disable-img="localSettings.disableImg"
              v-model:has-discord-hook="localSettings.hasDiscordHook"
              v-model:write-behind="localSettings.writeBehind"
            />
          </v-row>
        </div>
      </div>
    </template>
  </PageTemplate>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onBeforeUnmount } from 'vue'
import { useConfigStore } from '@/store/configStore'
import { useInitializedStore } from '@/store/initializedStore'
import type { AppConfig } from '@/api/config'
import PageTemplate from './PageLayout/PageTemplate.vue'
import ChangePassword from './Config/ChangePassword.vue'
import StorageAndSync from './Config/StorageAndSync.vue'
import FolderImport from './Config/FolderImport.vue'
import AdvancedConfig from './Config/AdvancedConfig.vue'
import { tryWithMessageStore } from '@/script/utils/try_catch'

const configStore = useConfigStore('mainId')
const initializedStore = useInitializedStore('mainId')

// UI State
const loading = ref(false)

// Local State
const localSettings = reactive<AppConfig>({
  readOnlyMode: false,
  disableImg: false,
  hasPassword: false,
  hasAuthKey: false,
  hasDiscordHook: false,
  authKey: '',
  discordHookUrl: '',
  syncPaths: [],
  address: '',

  port: 0,
  limits: {},
  writeBehind: {
    flushIntervalMs: 1000,
    softLimitMiB: 16,
    hardLimitMiB: 32
  }
})

const syncLocalWithStore = () => {
  if (configStore.config) {
    Object.assign(localSettings, JSON.parse(JSON.stringify(configStore.config)))
  }
}

const initData = async () => {
  loading.value = true
  const result = await tryWithMessageStore('mainId', async () => {
    await configStore.fetchConfig()
    syncLocalWithStore()
    return true
  })

  if (result === true) {
    initializedStore.initialized = true
  }

  loading.value = false
}

onMounted(initData)

onBeforeUnmount(() => {
  initializedStore.initialized = false
})
</script>

<style>
.config-stack {
  background-color: transparent;
}

.config-block {
  padding-bottom: 24px;
}

.config-block:last-child {
  padding-bottom: 0;
}

.config-block :deep(.v-col) {
  padding-bottom: 0;
  padding-top: 0;
}

.config-page-surface {
  background: transparent !important;
}
</style>
