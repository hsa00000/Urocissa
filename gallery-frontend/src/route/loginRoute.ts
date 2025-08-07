// src/router.ts

import { RouteRecordRaw } from 'vue-router'
import 'vue-router'

import LoginPage from '@/components/Page/LoginPage.vue'

export const loginRoute: RouteRecordRaw = {
  path: '/login',
  component: LoginPage,
  name: 'login',
  meta: {
    isReadPage: false,
    isViewPage: false,
    filterString: null,
    baseName: 'login',
    getParentPage: (route) => {
      return {
        name: 'home',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route) => {
      return {
        name: 'login',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    }
  }
}
