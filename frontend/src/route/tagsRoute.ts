// src/router.ts

import { RouteRecordRaw } from 'vue-router'
import 'vue-router'

import TagsPage from '@/components/Page/TagsPage.vue'

// ======================================
// 1. Define Simple Static Routes
// ======================================

export const tagsRoute: RouteRecordRaw = {
  path: '/tags',
  component: TagsPage,
  name: 'tags',
  meta: {
    level: 1,
    basicString: null,
    baseName: 'tags',
    getParentPage: (route) => {
      return {
        name: 'home',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route) => {
      return {
        name: 'tags',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    }
  }
}
