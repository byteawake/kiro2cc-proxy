// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useQuery } from '@tanstack/react-query'
import { getDashboard, type DashboardQuery } from '@/api/dashboard'

/** 数据看板：30s 轮询保持近实时；query 为 null（自定义区间未就绪）时不发请求 */
export function useDashboard(query: DashboardQuery | null) {
  return useQuery({
    queryKey: ['dashboard', query],
    queryFn: () => getDashboard(query as DashboardQuery),
    refetchInterval: 30_000,
    enabled: query !== null,
  })
}
