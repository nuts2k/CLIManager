import { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { useSettings } from "@/hooks/useSettings";
import { useProxyStatus } from "@/hooks/useProxyStatus";
import {
  refreshTrayMenu,
  proxySetGlobal,
  getClaudeSettingsOverlay,
  setClaudeSettingsOverlay,
} from "@/lib/tauri";
import type { ClaudeSettingsOverlayStorage } from "@/lib/tauri";
import i18n from "@/i18n";
import { AboutSection } from "@/components/settings/AboutSection";
import { useUpdater } from "@/components/updater/useUpdater";

interface SettingsPageProps {
  onBack: () => void;
  onShowImport?: () => void;
}

export function SettingsPage({ onBack, onShowImport }: SettingsPageProps) {
  const { t } = useTranslation();
  const { settings, updateSettings } = useSettings();
  const { proxyStatus, refresh: refreshProxyStatus } = useProxyStatus();

  // 设置页面独立的更新检查实例，仅用于内联显示更新状态
  const settingsUpdater = useUpdater();

  const currentLanguage = settings?.language ?? "zh";

  // 代理模式全局开关局部状态（用于乐观更新 + 失败回滚）
  const [proxyEnabled, setProxyEnabled] = useState(false);
  const [isProxyTogglePending, setIsProxyTogglePending] = useState(false);
  const proxyTogglePendingRef = useRef(false);

  // 当 proxyStatus 变化时同步 proxyEnabled
  useEffect(() => {
    if (proxyStatus && !proxyTogglePendingRef.current) {
      setProxyEnabled(proxyStatus.global_enabled);
    }
  }, [proxyStatus]);

  const handleProxyToggle = async (newValue: boolean) => {
    if (proxyTogglePendingRef.current) {
      return;
    }

    proxyTogglePendingRef.current = true;
    setIsProxyTogglePending(true);
    const previousValue = proxyEnabled;

    // 乐观更新
    setProxyEnabled(newValue);
    try {
      await proxySetGlobal(newValue);
      toast.success(
        newValue ? t("proxy.globalEnabled") : t("proxy.globalDisabledMsg"),
      );
    } catch (err) {
      // 回滚
      setProxyEnabled(previousValue);
      const errorStr = String(err);
      if (
        errorStr.includes("绑定失败") ||
        errorStr.includes("Address already in use") ||
        errorStr.includes("address already in use")
      ) {
        toast.error(t("proxy.portInUse", { port: "15800/15801" }));
      } else {
        toast.error(t("proxy.enableFailed") + ": " + errorStr);
      }
    } finally {
      proxyTogglePendingRef.current = false;
      setIsProxyTogglePending(false);
      void refreshProxyStatus();
    }
  };

  // Test config local state
  const [timeout, setTimeout] = useState<number>(
    settings?.test_config?.timeout_secs ?? 10,
  );
  const [testModel, setTestModel] = useState<string>(
    settings?.test_config?.test_model ?? "",
  );

  // Sync local state when settings load
  useEffect(() => {
    if (settings) {
      setTimeout(settings.test_config?.timeout_secs ?? 10);
      setTestModel(settings.test_config?.test_model ?? "");
    }
  }, [settings]);

  // Debounced test config save
  const debounceRef = useRef<ReturnType<typeof globalThis.setTimeout> | null>(
    null,
  );

  const saveTestConfig = useCallback(
    (newTimeout: number, newModel: string) => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
      debounceRef.current = globalThis.setTimeout(() => {
        updateSettings({
          test_config: {
            timeout_secs: newTimeout,
            test_model: newModel || null,
          },
        });
      }, 500);
    },
    [updateSettings],
  );

  const handleTimeoutChange = (value: string) => {
    const num = parseInt(value, 10);
    if (!isNaN(num) && num > 0) {
      setTimeout(num);
      saveTestConfig(num, testModel);
    }
  };

  const handleTestModelChange = (value: string) => {
    setTestModel(value);
    saveTestConfig(timeout, value);
  };

  const handleLanguageChange = async (lang: string) => {
    await i18n.changeLanguage(lang);
    await updateSettings({ language: lang });
    await refreshTrayMenu();
  };

  // Claude overlay 状态
  const [overlayJson, setOverlayJson] = useState<string>("");
  const [overlayInitialLoading, setOverlayInitialLoading] = useState<boolean>(true);
  const [overlayIsSaving, setOverlayIsSaving] = useState<boolean>(false);
  const [overlayStorageInfo, setOverlayStorageInfo] =
    useState<ClaudeSettingsOverlayStorage | null>(null);
  const [overlayLoadError, setOverlayLoadError] = useState<string | null>(null);
  const [overlaySaveError, setOverlaySaveError] = useState<string | null>(null);

  // 首次加载 overlay
  useEffect(() => {
    let cancelled = false;
    async function loadOverlay() {
      setOverlayInitialLoading(true);
      setOverlayLoadError(null);
      try {
        const state = await getClaudeSettingsOverlay();
        if (!cancelled) {
          setOverlayJson(state.overlay_json ?? "");
          setOverlayStorageInfo(state.storage);
        }
      } catch (err) {
        if (!cancelled) {
          setOverlayLoadError(t("settings.claudeOverlay.loadError") + ": " + String(err));
        }
      } finally {
        if (!cancelled) {
          setOverlayInitialLoading(false);
        }
      }
    }
    void loadOverlay();
    return () => {
      cancelled = true;
    };
  }, [t]);

  // 保存 overlay
  const handleOverlaySave = async () => {
    setOverlaySaveError(null);

    // 前端 JSON 校验
    if (overlayJson.trim() !== "") {
      let parsed: unknown;
      try {
        parsed = JSON.parse(overlayJson);
      } catch {
        setOverlaySaveError(t("settings.claudeOverlay.jsonInvalid"));
        return;
      }
      if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        setOverlaySaveError(t("settings.claudeOverlay.jsonMustBeObject"));
        return;
      }
    }

    setOverlayIsSaving(true);
    try {
      await setClaudeSettingsOverlay(overlayJson);
      // 保存成功后刷新状态
      const state = await getClaudeSettingsOverlay();
      setOverlayJson(state.overlay_json ?? "");
      setOverlayStorageInfo(state.storage);
      toast.success(t("settings.claudeOverlay.saveSuccess"));
    } catch (err) {
      setOverlaySaveError(t("settings.claudeOverlay.saveError") + ": " + String(err));
    } finally {
      setOverlayIsSaving(false);
    }
  };

  return (
    <div className="flex h-full flex-col">
      {/* Header with back button */}
      <div className="flex h-12 items-center gap-2 border-b border-border px-4">
        <Button variant="ghost" size="icon" onClick={onBack}>
          <ArrowLeft className="size-4" />
        </Button>
        <h2 className="text-base font-semibold">{t("settings.title")}</h2>
      </div>

      {/* Tabs 容器 */}
      <Tabs defaultValue="general" className="flex-1 flex flex-col overflow-hidden">
        <div className="px-6 pt-4">
          <TabsList variant="line">
            <TabsTrigger value="general">{t("settings.tabGeneral")}</TabsTrigger>
            <TabsTrigger value="advanced">{t("settings.tabAdvanced")}</TabsTrigger>
            <TabsTrigger value="about">{t("settings.tabAbout")}</TabsTrigger>
          </TabsList>
        </div>

        {/* 通用 Tab */}
        <TabsContent value="general" className="flex-1 overflow-auto p-6 space-y-6">
          {/* Language Section */}
          <section className="space-y-3">
            <h3 className="text-sm font-medium text-foreground">
              {t("settings.language")}
            </h3>
            <div className="max-w-xs">
              <Select value={currentLanguage} onValueChange={handleLanguageChange}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="zh">
                    {t("settings.languageZh")}
                  </SelectItem>
                  <SelectItem value="en">
                    {t("settings.languageEn")}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          </section>
        </TabsContent>

        {/* 高级 Tab */}
        <TabsContent value="advanced" className="flex-1 overflow-auto p-6 space-y-6">
          {/* 代理模式 Section */}
          <section className="space-y-3">
            <h3 className="text-sm font-medium text-foreground">
              {t("settings.proxyMode")}
            </h3>
            <div className="flex items-center justify-between max-w-xs">
              <p className="text-sm text-muted-foreground pr-4">
                {t("settings.proxyModeDescription")}
              </p>
              <Switch
                checked={proxyEnabled}
                disabled={isProxyTogglePending}
                onCheckedChange={handleProxyToggle}
              />
            </div>
          </section>

          <Separator />

          {/* Test Config Section */}
          <section className="space-y-3">
            <h3 className="text-sm font-medium text-foreground">
              {t("settings.testConfig")}
            </h3>
            <div className="grid max-w-xs gap-3">
              <div className="space-y-1.5">
                <Label htmlFor="timeout">{t("settings.timeout")}</Label>
                <Input
                  id="timeout"
                  type="number"
                  min={1}
                  value={timeout}
                  onChange={(e) => handleTimeoutChange(e.target.value)}
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="testModel">{t("settings.testModel")}</Label>
                <Input
                  id="testModel"
                  type="text"
                  placeholder="optional"
                  value={testModel}
                  onChange={(e) => handleTestModelChange(e.target.value)}
                />
              </div>
            </div>
          </section>

          {/* Import from CLI Config Section */}
          {onShowImport && (
            <>
              <Separator />
              <section className="space-y-3">
                <h3 className="text-sm font-medium text-foreground">
                  {t("import.settingsButton")}
                </h3>
                <Button variant="outline" onClick={onShowImport}>
                  {t("import.settingsButton")}
                </Button>
              </section>
            </>
          )}

          <Separator />

          {/* Claude Overlay Section */}
          <section className="space-y-4">
            <div className="space-y-1">
              <h3 className="text-sm font-medium text-foreground">
                {t("settings.claudeOverlay.title")}
              </h3>
              <p className="text-xs text-muted-foreground">
                {t("settings.claudeOverlay.description")}
              </p>
            </div>

            {/* 读取失败错误 */}
            {overlayLoadError && (
              <div className="rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {overlayLoadError}
              </div>
            )}

            {/* 编辑器 */}
            <div className="space-y-1.5">
              <Label htmlFor="overlayJson">
                {t("settings.claudeOverlay.editorLabel")}
              </Label>
              <Textarea
                id="overlayJson"
                className="min-h-[160px] font-mono text-xs"
                placeholder={t("settings.claudeOverlay.placeholder")}
                value={overlayInitialLoading ? "" : overlayJson}
                disabled={overlayInitialLoading || overlayIsSaving}
                onChange={(e) => {
                  setOverlayJson(e.target.value);
                  setOverlaySaveError(null);
                }}
                aria-invalid={overlaySaveError ? true : undefined}
              />
            </div>

            {/* 保存错误 */}
            {overlaySaveError && (
              <div className="rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {overlaySaveError}
              </div>
            )}

            {/* 保存按钮 */}
            <Button
              variant="outline"
              size="sm"
              disabled={overlayInitialLoading || overlayIsSaving}
              onClick={() => {
                void handleOverlaySave();
              }}
            >
              {overlayInitialLoading
                ? t("settings.claudeOverlay.loading")
                : overlayIsSaving
                  ? t("settings.claudeOverlay.saving")
                  : t("settings.claudeOverlay.saveButton")}
            </Button>

            {/* 存储位置信息 */}
            {overlayStorageInfo && (
              <div className="space-y-1 rounded-md border border-border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                <div className="flex gap-2">
                  <span className="font-medium text-foreground">
                    {t("settings.claudeOverlay.storageLocation")}:
                  </span>
                  <span>
                    {overlayStorageInfo.location === "icloud"
                      ? t("settings.claudeOverlay.locationIcloud")
                      : t("settings.claudeOverlay.locationLocal")}
                  </span>
                </div>
                <div className="flex gap-2">
                  <span className="font-medium text-foreground">
                    {t("settings.claudeOverlay.filePath")}:
                  </span>
                  <span className="break-all font-mono">
                    {overlayStorageInfo.file_path}
                  </span>
                </div>
                <div className="flex gap-2">
                  <span className="font-medium text-foreground">
                    {t("settings.claudeOverlay.syncEnabled")}:
                  </span>
                  <span>
                    {overlayStorageInfo.sync_enabled
                      ? t("settings.claudeOverlay.syncYes")
                      : t("settings.claudeOverlay.syncNo")}
                  </span>
                </div>
              </div>
            )}

            {/* 受保护字段说明 */}
            <div className="rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
              <p className="font-medium mb-1">
                {t("settings.claudeOverlay.protectedFieldsTitle")}
              </p>
              <p className="mb-1">{t("settings.claudeOverlay.protectedFieldsDesc")}</p>
              <ul className="list-disc pl-4 space-y-0.5 font-mono">
                <li>env.ANTHROPIC_AUTH_TOKEN</li>
                <li>env.ANTHROPIC_BASE_URL</li>
              </ul>
            </div>
          </section>
        </TabsContent>

        {/* 关于 Tab */}
        <TabsContent value="about" className="flex-1 overflow-auto p-6 space-y-6">
          <AboutSection
            onCheckUpdate={settingsUpdater.checkForUpdate}
            updateStatus={settingsUpdater.status}
            currentVersion={settingsUpdater.currentVersion}
            newVersion={settingsUpdater.newVersion}
            progress={settingsUpdater.progress}
            error={settingsUpdater.error}
            onUpdate={() => {
              void settingsUpdater.downloadAndInstall();
            }}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
}
