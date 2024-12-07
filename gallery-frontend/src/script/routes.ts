// src/router.ts

import { Component } from 'vue'
import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router'
import 'vue-router'

import TagsPage from '@/components/Page/TagsPage.vue'
import LoginPage from '@/components/LoginPage.vue'
import HomeMain from '@/components/Home/HomeMain.vue'
import AllPage from '@/components/Page/AllPage.vue'
import FavoritePage from '@/components/Page/FavoritePage.vue'
import ArchivedPage from '@/components/Page/ArchivedPage.vue'
import TrashedPage from '@/components/Page/TrashedPage.vue'
import AlbumsPage from '@/components/Page/AlbumsPage.vue'
import ViewPageMain from '@/components/Home/View/ViewPageMain.vue'
import HomeIsolated from '@/components/Home/HomeIsolated.vue'
import ViewPageIsolated from '@/components/Home/View/ViewPageIsolated.vue'

// ======================================
// 1. Define Simple Static Routes
// ======================================

const simpleRoutes: RouteRecordRaw[] = [
  {
    path: '/tags',
    component: TagsPage,
    name: 'TagsPage',
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: null,
      baseName: 'tags'
    }
  },
  {
    path: '/login',
    component: LoginPage,
    name: 'LoginPage',
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: null,
      baseName: 'login'
    }
  }
]

// ======================================
// 2. Define a Helper Function to Create Routes
// ======================================

/**
 * Creates a main route with an optional child route.
 *
 * @param path - The base path for the route.
 * @param component - The component to be rendered.
 * @param name - The unique name for the route.
 * @returns An array containing the RouteRecordRaw object.
 */
function createRoute(
  path: string,
  component: Component,
  name: string,
  basicString: string | null
): RouteRecordRaw[] {
  const mainRoute: RouteRecordRaw = {
    path: `/${path}`,
    component: component,
    name: name,
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: basicString,
      baseName: name
    },
    children: [
      {
        path: 'view/:hash',
        component: ViewPageMain,
        name: `${name}ViewPage`,
        meta: { isReadPage: false, isViewPage: true, basicString: basicString, baseName: name },
        children: [
          {
            path: 'read',
            component: HomeIsolated,
            name: `${name}ReadPage`,
            meta: { isReadPage: true, isViewPage: false, basicString: basicString, baseName: name },
            children: [
              {
                path: 'view/:subhash',
                name: `${name}ReadViewPage`,
                component: ViewPageIsolated,
                meta: {
                  isReadPage: true,
                  isViewPage: true,
                  basicString: basicString,
                  baseName: name
                }
              }
            ]
          }
        ]
      }
    ]
  }
  return [mainRoute]
}

// ======================================
// 3. Create Routes Using the Helper Function
// ======================================

const homePageRoutes = createRoute(
  '',
  HomeMain,
  'HomePage',
  'and(not(tag:"_archived"), not(tag:"_trashed"))'
)

const allPageRoutes = createRoute('all', AllPage, 'AllPage', null)

const favoritePageRoutes = createRoute(
  'favorite',
  FavoritePage,
  'FavoritePage',
  'and(tag:"_favorite", not(tag:"_trashed"))'
)

const archivedPageRoutes = createRoute(
  'archived',
  ArchivedPage,
  'ArchivedPage',
  'and(tag:"_archived", not(tag:"_trashed"))'
)

const trashedPageRoutes = createRoute('trashed', TrashedPage, 'TrashedPage', 'and(tag:"_trashed")')

const albumsPageRoutes = createRoute(
  'albums',
  AlbumsPage,
  'AlbumsPage',
  'and(type:"album", not(tag:"_trashed"))'
)

// ======================================
// 4. Combine All Routes
// ======================================

const routes: RouteRecordRaw[] = [
  ...simpleRoutes,
  ...homePageRoutes,
  ...allPageRoutes,
  ...favoritePageRoutes,
  ...archivedPageRoutes,
  ...trashedPageRoutes,
  ...albumsPageRoutes
]

// ======================================
// 5. Create and Export the Router Instance
// ======================================

const router = createRouter({
  history: createWebHistory(),
  routes
})

declare module 'vue-router' {
  interface RouteMeta {
    isReadPage: boolean
    isViewPage: boolean
    baseName: string
    basicString: string | null
  }
}

export default router
