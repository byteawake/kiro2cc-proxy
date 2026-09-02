// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import { Activity, RotateCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { FootSep, Metric, MetricFoot, MetricValue, MetricsBar } from '@/components/metrics'
import { PageHead } from '@/components/page-head'
import { Segmented, UpdatedAgo } from '@/components/toolbar'
import { CELL, PANEL, PANEL_TITLE, TH_BASE } from '@/components/table-kit'
import { useDashboard } from '@/hooks/use-dashboard'
import type { DashboardQuery } from '@/api/dashboard'
import { useApiKeys } from '@/hooks/use-credentials'
import { extractErrorMessage } from '@/lib/utils'
import { localeTag } from '@/lib/locale'
import type { DashboardBucket, DashboardSlice } from '@/types/api'

type RangeKey = '24' | '72' | '168' | '720'
type RangeMode = RangeKey | 'custom'

/** Date → datetime-local 输入值（浏览器本地时区） */
function toLocalInput(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}

const CHIP =
  'inline-flex h-[31px] flex-none items-center gap-[7px] rounded-[7px] border border-hairline-2 bg-surface px-2 font-mono text-[11.5px] tabular-nums text-ink-2 shadow-hair'

/** 大数缩写：1.2k / 3.4M（Tokens 列用，避免表格被撑爆） */
function compactTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 10_000) return `${(n / 1_000).toFixed(1)}k`
  return n.toLocaleString(localeTag())
}

function fmtCredits(n: number): string {
  return Number(n.toFixed(n >= 1 ? 2 : 4)).toLocaleString(localeTag(), {
    maximumFractionDigits: 4,
  })
}

/**
 * 数据看板（new-api 风格）：区间总量指标卡 + 请求/Credits 趋势 + 模型/账号/Key 维度切片。
 * 数据源为逐请求用量记录（api_key_usage.json），由后端 /api/admin/dashboard 实时聚合。
 */
export function DataDashboardPage() {
  const { t } = useTranslation()
  const [range, setRange] = useState<RangeMode>('168')
  const [startLocal, setStartLocal] = useState('')
  const [endLocal, setEndLocal] = useState('')
  const [apiKeyFilter, setApiKeyFilter] = useState('')

  // 自定义区间 → Unix 秒；未就绪/非法时返回 null 暂停查询
  const query: DashboardQuery | null = useMemo(() => {
    const api_key = apiKeyFilter === '' ? undefined : Number(apiKeyFilter)
    if (range === 'custom') {
      if (startLocal === '' || endLocal === '') return null
      const start = Math.floor(new Date(startLocal).getTime() / 1000)
      const end = Math.floor(new Date(endLocal).getTime() / 1000)
      if (!Number.isFinite(start) || !Number.isFinite(end) || start >= end) return null
      return { start, end, api_key }
    }
    return { hours: Number(range), api_key }
  }, [range, startLocal, endLocal, apiKeyFilter])

  const { data, isLoading, refetch, dataUpdatedAt } = useDashboard(query)
  const { data: apiKeys } = useApiKeys()
  const [trendMetric, setTrendMetric] = useState<'requests' | 'credits' | 'tokens'>('requests')

  const switchToCustom = () => {
    if (range !== 'custom') {
      if (startLocal === '' || endLocal === '') {
        const now = new Date()
        setStartLocal(toLocalInput(new Date(now.getTime() - 7 * 86_400_000)))
        setEndLocal(toLocalInput(now))
      }
      setRange('custom')
    }
  }

  const keyNames = new Map((apiKeys ?? []).map((k) => [k.id, k.name]))
  const resolveKeyName = (raw: string): string => {
    if (!raw.startsWith('#')) return raw
    const id = Number(raw.slice(1))
    return keyNames.get(id) ?? raw
  }

  const totals = data?.totals
  const series = data?.series ?? []
  const granularity = data?.granularity === 'day' ? t('dataDashboard.granDay') : t('dataDashboard.granHour')

  const handleRefresh = async () => {
    const result = await refetch()
    if (result.isError) {
      toast.error(extractErrorMessage(result.error))
      return
    }
    toast.success(t('dataDashboard.toastRefreshed'))
  }

  return (
    <div className="flex min-h-0 flex-col">
      <PageHead
        crumb={[t('dashboard.navMain'), t('dataDashboard.pageTitle')]}
        title={t('dataDashboard.pageTitle')}
        note={t('dataDashboard.headNote')}
        actions={
          <>
            <Segmented
              value={range}
              onChange={(v) => (v === 'custom' ? switchToCustom() : setRange(v))}
              groupLabel={t('dataDashboard.rangeGroupLabel')}
              options={[
                { key: '24', label: t('dataDashboard.range24h') },
                { key: '72', label: t('dataDashboard.range72h') },
                { key: '168', label: t('dataDashboard.range7d') },
                { key: '720', label: t('dataDashboard.range30d') },
                { key: 'custom', label: t('dataDashboard.custom') },
              ]}
            />
            {range === 'custom' && (
              <span className={CHIP}>
                <input
                  type="datetime-local"
                  aria-label={t('dataDashboard.startLabel')}
                  className="bg-transparent text-[11.5px] outline-none"
                  value={startLocal}
                  max={endLocal || undefined}
                  onChange={(e) => setStartLocal(e.target.value)}
                />
                <span className="text-ink-3">—</span>
                <input
                  type="datetime-local"
                  aria-label={t('dataDashboard.endLabel')}
                  className="bg-transparent text-[11.5px] outline-none"
                  value={endLocal}
                  min={startLocal || undefined}
                  onChange={(e) => setEndLocal(e.target.value)}
                />
              </span>
            )}
            <select
              aria-label={t('dataDashboard.keyFilterLabel')}
              className={`${CHIP} cursor-pointer appearance-none pr-2 [&_option]:bg-surface`}
              value={apiKeyFilter}
              onChange={(e) => setApiKeyFilter(e.target.value)}
            >
              <option value="">{t('dataDashboard.allKeys')}</option>
              {(apiKeys ?? []).map((k) => (
                <option key={k.id} value={String(k.id)}>
                  {k.name}
                </option>
              ))}
            </select>
            <UpdatedAgo dataUpdatedAt={dataUpdatedAt} />
            <Button variant="outline" onClick={handleRefresh} disabled={isLoading}>
              <RotateCw className={isLoading ? 'animate-spin' : ''} aria-hidden="true" />
              {t('common.refresh')}
            </Button>
          </>
        }
      />

      <MetricsBar>
        <Metric label={t('dataDashboard.metricRequests')}>
          <MetricValue value={(totals?.requests ?? 0).toLocaleString(localeTag())} />
          <MetricFoot className="truncate">
            {query
              ? query.start !== undefined
                ? `${new Date(query.start * 1000).toLocaleString(localeTag(), { dateStyle: 'short', timeStyle: 'short' })} — ${new Date((query.end ?? 0) * 1000).toLocaleString(localeTag(), { dateStyle: 'short', timeStyle: 'short' })}`
                : t('dataDashboard.rangeHours', { hours: query.hours ?? 0 })
              : t('dataDashboard.customPending')}
          </MetricFoot>
        </Metric>
        <Metric label={t('dataDashboard.metricTokens')}>
          <MetricValue value={compactTokens(totals?.inputTokens ?? 0)} unit={t('dataDashboard.tokensIn')} />
          <MetricFoot className="truncate">
            <span>
              {t('dataDashboard.tokensOut')}{' '}
              <b className="font-semibold text-ink-2">{compactTokens(totals?.outputTokens ?? 0)}</b>
            </span>
            <FootSep />
            <span>
              {t('dataDashboard.tokensCache')}{' '}
              <b className="font-semibold text-ink-2">
                {compactTokens((totals?.cacheReadTokens ?? 0) + (totals?.cacheCreationTokens ?? 0))}
              </b>
            </span>
          </MetricFoot>
        </Metric>
        <Metric label={t('dataDashboard.metricCredits')}>
          <MetricValue value={fmtCredits(totals?.credits ?? 0)} unit={t('dailyStats.unitCredits')} />
          <MetricFoot className="truncate">
            <span className="flex items-center gap-1">
              <Activity className="size-[13px] text-ink-3" aria-hidden="true" />
              {t('dataDashboard.liveMetering')}
            </span>
          </MetricFoot>
        </Metric>
        <Metric label={t('dataDashboard.metricCost')}>
          <MetricValue value={`$${(totals?.cost ?? 0).toFixed(2)}`} />
          <MetricFoot className="truncate">{t('dataDashboard.costNote')}</MetricFoot>
        </Metric>
      </MetricsBar>

      <section className={`${PANEL} mt-3 shrink-0`}>
        <div className="flex flex-wrap items-center gap-2.5 px-4 pt-[13px]">
          <div className="text-[13px] font-semibold tracking-[-.01em]">{t('dataDashboard.trendTitle')}</div>
          <div className="text-[11px] text-ink-3">
            {t('dataDashboard.trendSub', { granularity })}
          </div>
          <div className="ml-auto">
            <Segmented
              value={trendMetric}
              onChange={setTrendMetric}
              groupLabel={t('dataDashboard.trendMetricGroupLabel')}
              options={[
                { key: 'requests', label: t('dataDashboard.trendMetricRequests') },
                { key: 'credits', label: t('dataDashboard.trendMetricCredits') },
                { key: 'tokens', label: t('dataDashboard.trendMetricTokens') },
              ]}
            />
          </div>
        </div>
        <div className="px-4 pb-3.5 pt-2.5">
          <TrendChart series={series} metric={trendMetric} />
        </div>
      </section>

      <div className="mt-3 grid min-h-0 grid-cols-1 gap-3 lg:grid-cols-2">
        <SlicePanel
          title={t('dataDashboard.modelShare')}
          rows={data?.byModel ?? []}
          totalCredits={totals?.credits ?? 0}
          resolveName={(n) => n}
        />
        <SlicePanel
          title={t('dataDashboard.credShare')}
          rows={data?.byCredential ?? []}
          totalCredits={totals?.credits ?? 0}
          resolveName={(n) => n}
        />
      </div>

      <section className={`${PANEL} mt-3 shrink-0`}>
        <div className="px-4 pt-[13px]">
          <div className={PANEL_TITLE}>{t('dataDashboard.keysTitle')}</div>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-[12.5px]">
            <thead>
              <tr className="text-left text-[11px] text-ink-3">
                <th className={`${TH_BASE} w-[26%]`}>{t('dataDashboard.colName')}</th>
                <th className={`${TH_BASE} w-[14%]`}>{t('dataDashboard.colRequests')}</th>
                <th className={`${TH_BASE} w-[15%]`}>{t('dataDashboard.colTokensIn')}</th>
                <th className={`${TH_BASE} w-[15%]`}>{t('dataDashboard.colTokensOut')}</th>
                <th className={`${TH_BASE} w-[15%]`}>{t('dataDashboard.colCredits')}</th>
                <th className={TH_BASE}>{t('dataDashboard.colShare')}</th>
              </tr>
            </thead>
            <tbody>
              {(data?.byApiKey ?? []).slice(0, 10).map((row) => (
                <tr key={row.name}>
                  <td className={`${CELL} font-medium text-ink`}>{resolveKeyName(row.name)}</td>
                  <td className={`${CELL} tabular-nums text-ink-2`}>
                    {row.requests.toLocaleString(localeTag())}
                  </td>
                  <td className={`${CELL} tabular-nums text-ink-2`}>{compactTokens(row.inputTokens)}</td>
                  <td className={`${CELL} tabular-nums text-ink-2`}>{compactTokens(row.outputTokens)}</td>
                  <td className={`${CELL} tabular-nums text-ink-2`}>{fmtCredits(row.credits)}</td>
                  <td className={`${CELL} tabular-nums text-ink-3`}>
                    {sharePct(row.credits, totals?.credits ?? 0)}
                  </td>
                </tr>
              ))}
              {(data?.byApiKey ?? []).length === 0 && (
                <tr>
                  <td className={`${CELL} text-ink-3`} colSpan={6}>
                    {t('dataDashboard.empty')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  )
}

/** credits 占区间总额的百分比文案 */
function sharePct(credits: number, total: number): string {
  if (total <= 0) return '—'
  return `${((credits / total) * 100).toFixed(1)}%`
}

/** 趋势图：CSS 柱（请求次数）+ 非等比拉伸 SVG 折线（Credits），零图表库依赖 */
function TrendChart({
  series,
  metric,
}: {
  series: DashboardBucket[]
  metric: 'requests' | 'credits' | 'tokens'
}) {
  const { t } = useTranslation()
  if (series.length === 0) {
    return (
      <div className="grid h-[184px] place-items-center text-[12.5px] text-ink-3">
        {t('dataDashboard.empty')}
      </div>
    )
  }

  const valueOf = (s: DashboardBucket): number =>
    metric === 'requests' ? s.requests : metric === 'credits' ? s.credits : s.inputTokens + s.outputTokens
  const fmtValue = (v: number): string => (metric === 'credits' ? fmtCredits(v) : compactTokens(v))

  const maxValue = Math.max(...series.map(valueOf), metric === 'credits' ? 1e-9 : 1)
  // X 轴标签抽稀：目标 ≤12 个
  const labelStep = Math.max(1, Math.ceil(series.length / 12))

  const tooltip = (s: DashboardBucket): string => {
    if (metric === 'tokens') {
      return `${s.bucket} · ${t('dataDashboard.tokensIn')} ${compactTokens(s.inputTokens)} · ${t('dataDashboard.tokensOut')} ${compactTokens(s.outputTokens)}`
    }
    return `${s.bucket} · ${fmtValue(valueOf(s))}`
  }

  return (
    <div>
      {/* 单指标单轴：峰值是纵轴唯一参考值；Tokens 档附输入/输出图例 */}
      <div className="ml-[44px] flex items-center justify-between text-[10.5px] text-ink-3">
        <span className="tabular-nums">{t('dataDashboard.axisMax', { value: fmtValue(maxValue) })}</span>
        {metric === 'tokens' && (
          <span className="flex items-center gap-[10px] pr-1">
            <span className="flex items-center gap-[5px]">
              <i aria-hidden="true" className="block size-[9px] rounded-[2px]" style={{ background: 'var(--brand)' }} />
              {t('dataDashboard.tokensIn')}
            </span>
            <span className="flex items-center gap-[5px]">
              <i aria-hidden="true" className="block size-[9px] rounded-[2px]" style={{ background: 'var(--brand-hover)' }} />
              {t('dataDashboard.tokensOut')}
            </span>
          </span>
        )}
      </div>
      <div className="relative ml-[44px] flex h-[184px] items-end gap-[2px]">
        {series.map((s) => {
          const total = valueOf(s)
          const h = (total / maxValue) * 100
          const stacked = metric === 'tokens'
          const outH = stacked ? (s.outputTokens / maxValue) * 100 : 0
          const inH = stacked ? h - outH : h
          return (
            <div
              key={s.bucket}
              className="group relative flex h-full flex-1 cursor-default flex-col justify-end"
              title={tooltip(s)}
            >
              {stacked ? (
                <>
                  {s.outputTokens > 0 && (
                    <div
                      className="w-full rounded-t-[3px]"
                      style={{ height: `${Math.max(outH, 1)}%`, background: 'var(--brand-hover)' }}
                    />
                  )}
                  <div
                    className={`w-full transition-opacity group-hover:opacity-80 ${s.outputTokens > 0 ? '' : 'rounded-t-[3px]'}`}
                    style={{ height: `${Math.max(total > 0 ? 2 : 0, inH)}%`, background: 'var(--brand)' }}
                  />
                </>
              ) : (
                <div
                  className="w-full rounded-t-[3px] bg-brand transition-colors group-hover:bg-brand-hover"
                  style={{ height: `${Math.max(total > 0 ? 2 : 0, h)}%` }}
                />
              )}
            </div>
          )
        })}
      </div>
      <div className="ml-[44px] mt-[6px] flex gap-[2px]">
        {series.map((s, i) => (
          <div key={s.bucket} className="flex-1 overflow-visible text-center">
            {i % labelStep === 0 && (
              <span className="block whitespace-nowrap font-mono text-[10px] tabular-nums text-ink-3">
                {s.bucket}
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  )
}

/** 维度切片面板：名称 + 请求/Tokens/Credits 数值 + credits 份额条 */
function SlicePanel({
  title,
  rows,
  totalCredits,
  resolveName,
}: {
  title: string
  rows: DashboardSlice[]
  totalCredits: number
  resolveName: (name: string) => string
}) {
  const { t } = useTranslation()
  const top = rows.slice(0, 6)
  const maxCredits = Math.max(...top.map((r) => r.credits), 1e-9)

  return (
    <section className={`${PANEL} shrink-0`}>
      <div className="px-4 pt-[13px]">
        <div className={PANEL_TITLE}>{title}</div>
      </div>
      {top.length === 0 ? (
        <div className="grid h-[120px] place-items-center px-4 text-[12.5px] text-ink-3">
          {t('dataDashboard.empty')}
        </div>
      ) : (
        <div className="flex flex-col gap-2 px-4 pb-3.5 pt-1">
          {top.map((r) => (
            <div key={r.name}>
              <div className="flex items-baseline justify-between gap-2 text-[12px]">
                <span className="truncate font-medium text-ink" title={r.name}>
                  {resolveName(r.name)}
                </span>
                <span className="flex-none tabular-nums text-ink-3">
                  {r.requests.toLocaleString(localeTag())} · {compactTokens(r.inputTokens + r.outputTokens)} ·{' '}
                  <b className="font-semibold text-ink-2">{fmtCredits(r.credits)}</b>
                </span>
              </div>
              <div className="mt-[4px] h-[4px] overflow-hidden rounded-[2px] bg-surface-3">
                <div
                  className="h-full rounded-[2px] bg-brand"
                  style={{ width: `${Math.max((r.credits / maxCredits) * 100, r.credits > 0 ? 2 : 0)}%` }}
                />
              </div>
            </div>
          ))}
          <div className="text-right text-[10.5px] text-ink-3">
            {t('dataDashboard.shareOf', {
              count: top.length,
              percent: sharePct(
                top.reduce((s, r) => s + r.credits, 0),
                totalCredits
              ),
            })}
          </div>
        </div>
      )}
    </section>
  )
}
