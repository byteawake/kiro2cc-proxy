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

/** 看板查询参数：预设 hours 或自定义 start/end（Unix 秒），可选按 Key 过滤。
 *  字段名与后端 Query 参数一一对应（api_key 为下划线，勿改驼峰） */
export interface DashboardQuery {
  hours?: number
  start?: number
  end?: number
  api_key?: number
}

export async function getDashboard(query: DashboardQuery): Promise<DashboardResponse> {
  const { data } = await api.get<DashboardResponse>('/dashboard', { params: query })
  return data
}
