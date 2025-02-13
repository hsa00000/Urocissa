// src/router.ts

import { Component } from 'vue'
import { createRouter, createWebHistory, LocationQuery, RouteRecordRaw } from 'vue-router'
import 'vue-router'

import TagsPage from '@/components/Page/TagsPage.vue'
import LoginPage from '@/components/LoginPage.vue'
import HomeMain from '@/components/Home/Page/HomeMain.vue'
import AllPage from '@/components/Page/AllPage.vue'
import FavoritePage from '@/components/Page/FavoritePage.vue'
import ArchivedPage from '@/components/Page/ArchivedPage.vue'
import TrashedPage from '@/components/Page/TrashedPage.vue'
import AlbumsPage from '@/components/Page/AlbumsPage.vue'
import ViewPageMain from '@/components/Home/View/Page/ViewPageMain.vue'
import HomeIsolated from '@/components/Home/Page/HomeIsolated.vue'
import ViewPageIsolated from '@/components/Home/View/Page/ViewPageIsolated.vue'
import VideosPage from '@/components/Page/VideosPage.vue'

// ======================================
// 1. Define Simple Static Routes
// ======================================

const simpleRoutes: RouteRecordRaw[] = [
  { path: '/', redirect: '/home' },
  {
    path: '/tags',
    component: TagsPage,
    name: 'TagsPage',
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: null,
      baseName: 'tags',
      getParentPage: (route) => {
        return {
          name: 'HomePage',
          params: { hash: undefined },
          query: route.query
        }
      }
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
      baseName: 'login',
      getParentPage: (route) => {
        return {
          name: 'HomePage',
          params: { hash: undefined },
          query: route.query
        }
      }
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
  baseName: BaseName,
  component: Component,
  name: string,
  basicString: string | null
): RouteRecordRaw[] {
  const mainRoute: RouteRecordRaw = {
    path: `/${baseName}`,
    component: component,
    name: name,
    meta: {
      isReadPage: false,
      isViewPage: false,
      basicString: basicString,
      baseName: baseName,
      getParentPage: (route) => {
        return {
          name: name,
          params: { hash: undefined },
          query: route.query
        }
      }
    },
    children: [
      {
        path: 'view/:hash',
        component: ViewPageMain,
        name: `${name}ViewPage`,
        meta: {
          isReadPage: false,
          isViewPage: true,
          basicString: basicString,
          baseName: baseName,
          getParentPage: (route) => {
            return {
              name: name,
              params: { hash: undefined },
              query: route.query
            }
          }
        },
        children: [
          {
            path: 'read',
            component: HomeIsolated,
            name: `${name}ReadPage`,
            meta: {
              isReadPage: true,
              isViewPage: false,
              basicString: basicString,
              baseName: baseName,
              getParentPage: (route) => {
                return {
                  name: `${name}ViewPage`,
                  params: { hash: route.params.hash },
                  query: route.query
                }
              }
            },
            children: [
              {
                path: 'view/:subhash',
                name: `${name}ReadViewPage`,
                component: ViewPageIsolated,
                meta: {
                  isReadPage: true,
                  isViewPage: true,
                  basicString: basicString,
                  baseName: baseName,
                  getParentPage: (route) => {
                    return {
                      name: `${name}ReadPage`,
                      params: { hash: route.params.hash },
                      query: route.query
                    }
                  }
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
  'home',
  HomeMain,
  'HomePage',
  'and(not(tag:"_archived"), not(tag:"_trashed"))'
)

const allPageRoutes = createRoute('all', AllPage, 'AllPage', 'not(tag:"_trashed")')

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

const videosPageRoutes = createRoute(
  'videos',
  VideosPage,
  'VideosPage',
  'and(type:"video", not(tag:"_archived"), not(tag:"_trashed"))'
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
  ...albumsPageRoutes,
  ...videosPageRoutes
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
    baseName: BaseName
    basicString: string | null
    getParentPage: (router: RouteLocationNormalizedLoadedGeneric) => ParentPageReturnType
  }
}

export default router

interface ParentPageReturnType {
  name: string
  params: { hash: string | string[] | undefined }
  query: LocationQuery
}

type BaseName =
  | 'home'
  | 'all'
  | 'favorite'
  | 'archived'
  | 'trashed'
  | 'albums'
  | 'videos'
  | 'album'
  | 'tags'
  | 'login'
