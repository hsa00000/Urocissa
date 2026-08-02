import axios from 'axios'
import { routeResourceSnapshotSchema } from '@/type/schemas'
import type { RouteResourceSnapshot } from '@/type/types'

const ROUTE_RESOURCE_TIMEOUT_MS = 15_000

export async function fetchRouteResource(
  resourceId: string,
  signal?: AbortSignal
): Promise<RouteResourceSnapshot> {
  const response = await axios.get(`/get/resource/${encodeURIComponent(resourceId)}`, {
    signal,
    timeout: ROUTE_RESOURCE_TIMEOUT_MS,
    headers: {
      'x-silent-route-resource-error': 'true'
    }
  })
  return routeResourceSnapshotSchema.parse(response.data)
}
