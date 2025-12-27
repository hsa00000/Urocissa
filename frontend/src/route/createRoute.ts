// src/router.ts

import { Component } from 'vue'
import { RouteRecordRaw } from 'vue-router'
import 'vue-router'

import ViewPageMain from '@/components/View/ViewPageMain.vue'
import HomeIsolated from '@/components/Home/HomeIsolated.vue'
import ViewPageIsolated from '@/components/View/ViewPageIsolated.vue'

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
  | 'share'
  | 'links'

// ======================================
// Define a Helper Function to Create Routes
// ======================================

/**
 * Creates a main route with an optional child route.
 *
 * @param path - The base path for the route.
 * @param component - The component to be rendered.
 * @param name - The unique name for the route.
 * @returns An array containing the RouteRecordRaw object.
 */
export function createRoute(baseName: BaseName, component: Component): RouteRecordRaw[] {
  const mainRoute: RouteRecordRaw = {
    path: `/${baseName}`,
    component: component,
    name: baseName,
    meta: {
      level: 1,
      baseName: baseName,
      getParentPage: (route) => {
        return {
          name: baseName,
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      },
      getChildPage: (route, hash) => {
        return {
          name: `${baseName}ViewPage`,
          params: { hash: hash, subhash: undefined },
          query: route.query
        }
      }
    },
    children: [
      {
        path: 'view/:hash',
        component: ViewPageMain,
        name: `${baseName}ViewPage`,
        meta: {
          level: 2,
          baseName: baseName,
          getParentPage: (route) => {
            return {
              name: baseName,
              params: { hash: undefined, subhash: undefined },
              query: route.query
            }
          },
          getChildPage: (route) => {
            return {
              name: `${baseName}ReadPage`,
              params: { hash: route.params.hash, subhash: undefined },
              query: route.query
            }
          }
        },
        children: [
          {
            path: 'read',
            component: HomeIsolated,
            name: `${baseName}ReadPage`,
            meta: {
              level: 3,
              baseName: baseName,
              getParentPage: (route) => {
                return {
                  name: `${baseName}ViewPage`,
                  params: { hash: route.params.hash, subhash: undefined },
                  query: route.query
                }
              },
              getChildPage: (route, subhash) => {
                return {
                  name: `${baseName}ReadViewPage`,
                  params: { hash: route.params.hash, subhash: subhash },
                  query: route.query
                }
              }
            },
            children: [
              {
                path: 'view/:subhash',
                name: `${baseName}ReadViewPage`,
                component: ViewPageIsolated,
                meta: {
                  level: 4,
                  baseName: baseName,
                  getParentPage: (route) => {
                    return {
                      name: `${baseName}ReadPage`,
                      params: { hash: route.params.hash, subhash: undefined },
                      query: route.query
                    }
                  },
                  getChildPage: (route) => {
                    return {
                      name: `${baseName}ReadViewPage`,
                      params: { hash: route.params.hash, subhash: route.params.subhash },
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
