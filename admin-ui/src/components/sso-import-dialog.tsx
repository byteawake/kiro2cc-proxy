// Copyright (c) 2026 Harllan He. Licensed under MIT.
import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { CheckCircle2, Copy, ExternalLink, Loader2, XCircle } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useStartSsoSession, useCancelSsoSession, pollSsoSession } from '@/hooks/use-credentials'
import { extractErrorMessage } from '@/lib/utils'
import type { SsoSessionResponse } from '@/types/api'

interface SsoImportDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

type ApiRegion = 'us-east-1' | 'eu-central-1'

/** 对话框字段标签：字号对齐设计稿体系；中文标签不套 uppercase（与添加账号对话框一致） */
const LABEL = 'text-[11.5px] font-medium text-ink-2'

/** 状态提示文案 key（失败类状态共用红色块） */
const FAILURE_STATUS_KEY: Partial<Record<SsoSessionResponse['status'], string>> = {
  failed: 'credentials.sso.statusFailed',
  expired: 'credentials.sso.statusExpired',
  denied: 'credentials.sso.statusDenied',
  cancelled: 'credentials.sso.statusCancelled',
}

export function SsoImportDialog({ open, onOpenChange }: SsoImportDialogProps) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()

  // ==== 表单 ====
  const [startUrl, setStartUrl] = useState('')
  const [authRegion, setAuthRegion] = useState('')
  const [apiRegion, setApiRegion] = useState<ApiRegion>('us-east-1')
  const [priority, setPriority] = useState('0')
  const [email, setEmail] = useState('')
  const [nickname, setNickname] = useState('')
  const [proxyUrl, setProxyUrl] = useState('')
  const [proxyUsername, setProxyUsername] = useState('')
  const [proxyPassword, setProxyPassword] = useState('')

  // ==== 会话 ====
  const [session, setSession] = useState<SsoSessionResponse | null>(null)
  // verificationUriComplete 只自动打开一次，之后提供「重新打开」按钮
  const autoOpenedRef = useRef(false)

  const { mutate: startMutate, isPending: starting } = useStartSsoSession()
  const { mutate: cancelMutate, isPending: cancelling } = useCancelSsoSession()

  const resetForm = () => {
    setStartUrl('')
    setAuthRegion('')
    setApiRegion('us-east-1')
    setPriority('0')
    setEmail('')
    setNickname('')
    setProxyUrl('')
    setProxyUsername('')
    setProxyPassword('')
    setSession(null)
    autoOpenedRef.current = false
  }

  const isPolling = session?.status === 'pending'

  // 会话进行中每 3s 轮询一次状态；结束后刷新账号列表（导入在服务端完成）
  useEffect(() => {
    if (!session || !isPolling) return
    const timer = window.setTimeout(async () => {
      try {
        const updated = await pollSsoSession(session.sessionId)
        if (updated.status !== 'pending') {
          queryClient.invalidateQueries({ queryKey: ['credentials'] })
        }
        setSession(updated)
      } catch (error) {
        // 单次轮询失败不终止流程，下个周期重试
        console.warn('SSO 会话轮询失败', extractErrorMessage(error))
      }
    }, 3000)
    return () => window.clearTimeout(timer)
  }, [session, isPolling, queryClient])

  // 关闭且会话已结束时清空状态；等待中关闭则保留后台继续轮询（下次打开可续看）
  useEffect(() => {
    if (!open && session && session.status !== 'pending') {
      resetForm()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open])

  const handleCopyUserCode = async () => {
    if (!session) return
    try {
      await navigator.clipboard.writeText(session.userCode)
      toast.success(t('common.copied'))
    } catch {
      toast.error(t('common.copyFailed'))
    }
  }

  const handleOpenAuthPage = () => {
    const url = session?.verificationUriComplete || session?.verificationUri
    if (url) window.open(url, '_blank', 'noopener,noreferrer')
  }

  const handleStart = (e: React.FormEvent) => {
    e.preventDefault()
    if (!startUrl.trim()) {
      toast.error(t('credentials.sso.toastStartUrlRequired'))
      return
    }
    if (!authRegion.trim()) {
      toast.error(t('credentials.sso.toastRegionRequired'))
      return
    }

    autoOpenedRef.current = false
    startMutate(
      {
        startUrl: startUrl.trim(),
        authRegion: authRegion.trim(),
        apiRegion,
        priority: parseInt(priority) || 0,
        email: email.trim() || undefined,
        nickname: nickname.trim() || undefined,
        proxyUrl: proxyUrl.trim() || undefined,
        proxyUsername: proxyUsername.trim() || undefined,
        proxyPassword: proxyPassword.trim() || undefined,
      },
      {
        onSuccess: (created) => {
          setSession(created)
          // 自动打开授权页（浏览器可能拦截弹窗，下方始终提供手动打开按钮）
          const url = created.verificationUriComplete || created.verificationUri
          if (url) window.open(url, '_blank', 'noopener,noreferrer')
          autoOpenedRef.current = true
        },
        onError: (error: unknown) => {
          toast.error(t('credentials.sso.toastStartFailed', { message: extractErrorMessage(error) }))
        },
      }
    )
  }

  const handleCancelSession = () => {
    if (!session) return
    cancelMutate(session.sessionId, {
      onSuccess: (updated) => setSession(updated),
      onError: (error: unknown) => {
        toast.error(t('credentials.sso.toastCancelFailed', { message: extractErrorMessage(error) }))
      },
    })
  }

  const verificationUrl = session?.verificationUriComplete || session?.verificationUri

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{t('credentials.sso.title')}</DialogTitle>
        </DialogHeader>

        {!session ? (
          <form onSubmit={handleStart} className="flex flex-col min-h-0 flex-1">
            <div className="space-y-4 py-4 overflow-y-auto flex-1 pr-1">
              {/* Start URL */}
              <div className="space-y-2">
                <label htmlFor="ssoStartUrl" className={LABEL}>
                  {t('credentials.sso.startUrlLabel')} <span className="text-danger">*</span>
                </label>
                <Input
                  id="ssoStartUrl"
                  type="text"
                  placeholder={t('credentials.sso.startUrlPlaceholder')}
                  value={startUrl}
                  onChange={(e) => setStartUrl(e.target.value)}
                  disabled={starting}
                />
                <p className="text-[11px] leading-[1.55] text-ink-3">
                  {t('credentials.sso.startUrlHint')}
                </p>
              </div>

              {/* Region 配置 */}
              <div className="space-y-2">
                <label htmlFor="ssoApiRegion" className={LABEL}>
                  {t('credentials.regionConfigLabel')}
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <Input
                    id="ssoAuthRegion"
                    placeholder={t('credentials.authRegionPlaceholder')}
                    value={authRegion}
                    onChange={(e) => setAuthRegion(e.target.value)}
                    disabled={starting}
                  />
                  <select
                    id="ssoApiRegion"
                    value={apiRegion}
                    onChange={(e) => setApiRegion(e.target.value as ApiRegion)}
                    disabled={starting}
                    className="h-[31px] w-full rounded-[7px] border border-hairline-2 bg-surface-2 px-2.5 text-[12px] text-ink outline-none transition-colors focus:border-brand disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    <option value="us-east-1">us-east-1</option>
                    <option value="eu-central-1">eu-central-1</option>
                  </select>
                </div>
                <p className="text-[11px] leading-[1.55] text-ink-3">
                  {t('credentials.sso.regionHint')}
                </p>
              </div>

              {/* 优先级 */}
              <div className="space-y-2">
                <label htmlFor="ssoPriority" className={LABEL}>
                  {t('credentials.priorityFieldLabel')}
                </label>
                <Input
                  id="ssoPriority"
                  type="number"
                  min="0"
                  value={priority}
                  onChange={(e) => setPriority(e.target.value)}
                  disabled={starting}
                />
                <p className="text-[11px] leading-[1.55] text-ink-3">
                  {t('credentials.priorityHint')}
                </p>
              </div>

              {/* 可选标识信息：与添加账号对话框字段一致 */}
              <div className="space-y-2">
                <label htmlFor="ssoEmail" className={LABEL}>
                  {t('credentials.emailLabel')}
                </label>
                <Input
                  id="ssoEmail"
                  type="text"
                  placeholder={t('credentials.emailPlaceholderAdd')}
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={starting}
                />
              </div>

              <div className="space-y-2">
                <label htmlFor="ssoNickname" className={LABEL}>
                  {t('credentials.nicknameLabel')}
                </label>
                <Input
                  id="ssoNickname"
                  type="text"
                  placeholder={t('credentials.nicknamePlaceholder')}
                  value={nickname}
                  onChange={(e) => setNickname(e.target.value)}
                  disabled={starting}
                />
              </div>

              {/* 代理配置 */}
              <div className="space-y-2">
                <label className={LABEL}>{t('credentials.proxyConfigLabel')}</label>
                <Input
                  id="ssoProxyUrl"
                  placeholder={t('credentials.proxyUrlPlaceholderAdd')}
                  value={proxyUrl}
                  onChange={(e) => setProxyUrl(e.target.value)}
                  disabled={starting}
                />
                <div className="grid grid-cols-2 gap-2">
                  <Input
                    id="ssoProxyUsername"
                    placeholder={t('credentials.proxyUsernamePlaceholder')}
                    value={proxyUsername}
                    onChange={(e) => setProxyUsername(e.target.value)}
                    disabled={starting}
                  />
                  <Input
                    id="ssoProxyPassword"
                    type="password"
                    placeholder={t('credentials.proxyPasswordPlaceholder')}
                    value={proxyPassword}
                    onChange={(e) => setProxyPassword(e.target.value)}
                    disabled={starting}
                  />
                </div>
                <p className="text-[11px] leading-[1.55] text-ink-3">
                  {t('credentials.proxyHintAdd')}
                </p>
              </div>
            </div>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
                disabled={starting}
              >
                {t('common.cancel')}
              </Button>
              <Button type="submit" disabled={starting}>
                {starting ? t('credentials.sso.starting') : t('credentials.sso.start')}
              </Button>
            </DialogFooter>
          </form>
        ) : (
          <div className="flex flex-col min-h-0 flex-1">
            <div className="space-y-4 py-4 overflow-y-auto flex-1 pr-1">
              {/* 锁定参数展示（只读） */}
              <div className="rounded-md border border-hairline-2 bg-surface-2 p-3 space-y-1">
                <div className="flex justify-between gap-2 text-[12px]">
                  <span className="text-ink-3">Start URL</span>
                  <span className="font-mono truncate max-w-[60%]" title={session.startUrl}>
                    {session.startUrl}
                  </span>
                </div>
                <div className="flex justify-between gap-2 text-[12px]">
                  <span className="text-ink-3">Auth Region</span>
                  <span className="font-mono">{session.authRegion}</span>
                </div>
                <div className="flex justify-between gap-2 text-[12px]">
                  <span className="text-ink-3">API Region</span>
                  <span className="font-mono">{session.apiRegion}</span>
                </div>
                <p className="text-[11px] leading-[1.55] text-ink-3 pt-1">
                  {t('credentials.sso.lockedHint')}
                </p>
              </div>

              {/* 等待授权 */}
              {session.status === 'pending' && (
                <div className="space-y-3">
                  <div className="space-y-2">
                    <label className={LABEL}>{t('credentials.sso.userCodeLabel')}</label>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 rounded-md border border-hairline-2 bg-surface-2 px-3 py-2 text-lg font-mono tracking-widest text-center">
                        {session.userCode}
                      </code>
                      <Button
                        type="button"
                        variant="outline"
                        size="icon"
                        onClick={handleCopyUserCode}
                        aria-label={t('common.copy')}
                      >
                        <Copy />
                      </Button>
                    </div>
                    <p className="text-[11px] leading-[1.55] text-ink-3">
                      {t('credentials.sso.userCodeHint')}
                    </p>
                  </div>

                  {verificationUrl && (
                    <Button
                      type="button"
                      variant="outline"
                      className="w-full"
                      onClick={handleOpenAuthPage}
                    >
                      <ExternalLink />
                      {t('credentials.sso.reopenAuthPage')}
                    </Button>
                  )}

                  <div className="flex items-center gap-2 text-[12px] text-ink-3">
                    <Loader2 className="animate-spin" />
                    {t('credentials.sso.waiting')}
                  </div>
                </div>
              )}

              {/* 成功 */}
              {session.status === 'completed' && (
                <div className="flex items-start gap-3 rounded-md border border-ok-line bg-ok-soft p-3">
                  <CheckCircle2 className="shrink-0 mt-0.5 text-ok" />
                  <div className="text-[12px]">
                    <div className="font-medium">{t('credentials.sso.completedTitle')}</div>
                    <div className="text-ink-3 mt-1">
                      {t('credentials.sso.completedDesc', {
                        credentialId: session.credentialId ?? '-',
                      })}
                      {session.email && <span className="ml-1">{session.email}</span>}
                    </div>
                  </div>
                </div>
              )}

              {/* 失败 / 超时 / 拒绝 / 取消 */}
              {FAILURE_STATUS_KEY[session.status] && (
                <div className="flex items-start gap-3 rounded-md border border-danger-line bg-danger-soft p-3">
                  <XCircle className="shrink-0 mt-0.5 text-danger" />
                  <div className="text-[12px]">
                    <div className="font-medium">{t(FAILURE_STATUS_KEY[session.status]!)}</div>
                    {session.error && (
                      <div className="text-ink-3 mt-1 break-all">{session.error}</div>
                    )}
                  </div>
                </div>
              )}
            </div>

            <DialogFooter>
              {isPolling && (
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleCancelSession}
                  disabled={cancelling}
                >
                  {cancelling ? t('common.loading') : t('credentials.sso.cancelSession')}
                </Button>
              )}
              {!isPolling && session.status !== 'completed' && (
                <Button type="button" variant="outline" onClick={resetForm}>
                  {t('credentials.sso.retry')}
                </Button>
              )}
              <Button
                type="button"
                onClick={() => onOpenChange(false)}
                disabled={cancelling}
              >
                {t('credentials.sso.done')}
              </Button>
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
