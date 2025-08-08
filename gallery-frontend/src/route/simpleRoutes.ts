import { RouteRecordRaw } from 'vue-router'
import 'vue-router'

import LinksPage from '@/components/Page/LinksPage.vue'
import LoginPage from '@/components/Page/LoginPage.vue'
import TagsPage from '@/components/Page/TagsPage.vue'

const linksRoute: RouteRecordRaw = {
  path: '/links',
  component: LinksPage,
  name: 'links',
  meta: {
    isReadPage: false,
    isViewPage: false,
    filterString: null,
    baseName: 'links',
    getParentPage: (route) => {
      return {
        name: 'home',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route) => {
      return {
        name: 'links',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    }
  }
}

const loginRoute: RouteRecordRaw = {
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

const tagsRoute: RouteRecordRaw = {
  path: '/tags',
  component: TagsPage,
  name: 'tags',
  meta: {
    isReadPage: false,
    isViewPage: false,
    filterString: null,
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

export const simpleRoutes: RouteRecordRaw[] = [tagsRoute, linksRoute, loginRoute]
