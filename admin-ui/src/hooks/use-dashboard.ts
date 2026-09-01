// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useQuery } from '@tanstack/react-query'
import { getDashboard } from '@/api/dashboard'

/** 数据看板：30s 轮询保持近实时 */
export function useDashboard(hours: number) {
  return useQuery({
    queryKey: ['dashboard', hours],
    queryFn: () => getDashboard(hours),
    refetchInterval: 30_000,
  })
}
