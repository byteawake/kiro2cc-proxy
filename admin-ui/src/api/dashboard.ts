// Copyright (c) 2026 Harllan He. Licensed under MIT.
import axios from 'axios'
import { storage } from '@/lib/storage'
import type { DashboardResponse } from '@/types/api'

const api = axios.create({
  baseURL: '/api/admin',
  headers: {
    'Content-Type': 'application/json',
  },
})

api.interceptors.request.use((config) => {
  const apiKey = storage.getApiKey()
  if (apiKey) {
    config.headers['x-api-key'] = apiKey
  }
  return config
})

/** 数据看板聚合快照（hours: 统计最近 N 小时，1-720） */
export async function getDashboard(hours: number): Promise<DashboardResponse> {
  const { data } = await api.get<DashboardResponse>('/dashboard', {
    params: { hours },
  })
  return data
}
