import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Eye, EyeOff, Loader2, X } from "lucide-react";
import { z } from "zod/v4";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { Provider, ProtocolType } from "@/types/provider";

interface ProviderDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mode: "create" | "edit";
  provider?: Provider | null;
  cliId: string;
  onSave: (data: ProviderFormData) => Promise<void>;
}

export interface ModelMapEntry {
  source: string;
  target: string;
}

export interface ProviderFormData {
  name: string;
  apiKey: string;
  baseUrl: string;
  model: string;
  testModel: string;
  protocolType: ProtocolType;
  notes: string;
  haikuModel: string;
  sonnetModel: string;
  opusModel: string;
  reasoningEffort: string;
  upstreamModel: string;
  upstreamModelMap: ModelMapEntry[];
}

const formSchema = z.object({
  name: z.string().min(1),
  apiKey: z.string().min(1),
  baseUrl: z.string().min(1),
});

function getSuggestedTestModel(protocolType: ProtocolType): string {
  return protocolType === "anthropic" ? "claude-sonnet-4-6" : "gpt-5.2";
}

function getSuggestedUpstreamModel(protocolType: ProtocolType): string {
  return protocolType === "anthropic" ? "" : "gpt-5.2";
}

export function ProviderDialog({
  open,
  onOpenChange,
  mode,
  provider,
  cliId: _cliId,
  onSave,
}: ProviderDialogProps) {
  const { t } = useTranslation();
  const [saving, setSaving] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  const [form, setForm] = useState<ProviderFormData>({
    name: "",
    apiKey: "",
    baseUrl: "",
    model: "",
    testModel: getSuggestedTestModel("anthropic"),
    protocolType: "anthropic",
    notes: "",
    haikuModel: "",
    sonnetModel: "",
    opusModel: "",
    reasoningEffort: "",
    upstreamModel: "",
    upstreamModelMap: [],
  });

  useEffect(() => {
    if (open) {
      setErrors({});
      setShowApiKey(false);
      if (mode === "edit" && provider) {
        // 旧值兼容：open_ai_compatible 自动映射为 open_ai_chat_completions
        const protocolType: ProtocolType =
          (provider.protocol_type as string) === "open_ai_compatible"
            ? "open_ai_chat_completions"
            : provider.protocol_type;

        // Record<string, string> → ModelMapEntry[]
        const upstreamModelMap: ModelMapEntry[] = provider.upstream_model_map
          ? Object.entries(provider.upstream_model_map).map(
              ([source, target]) => ({ source, target }),
            )
          : [];

        setForm({
          name: provider.name,
          apiKey: provider.api_key,
          baseUrl: provider.base_url,
          model: provider.model,
          testModel: provider.test_model ?? "",
          protocolType,
          notes: provider.notes ?? "",
          haikuModel: provider.model_config?.haiku_model ?? "",
          sonnetModel: provider.model_config?.sonnet_model ?? "",
          opusModel: provider.model_config?.opus_model ?? "",
          reasoningEffort: provider.model_config?.reasoning_effort ?? "",
          upstreamModel: provider.upstream_model ?? "",
          upstreamModelMap,
        });
      } else {
        setForm({
          name: "",
          apiKey: "",
          baseUrl: "",
          model: "",
          testModel: getSuggestedTestModel("anthropic"),
          protocolType: "anthropic",
          notes: "",
          haikuModel: "",
          sonnetModel: "",
          opusModel: "",
          reasoningEffort: "",
          upstreamModel: "",
          upstreamModelMap: [],
        });
      }
    }
  }, [open, mode, provider]);

  const updateProtocolType = (nextProtocolType: ProtocolType) => {
    setForm((prev) => {
      const prevSuggestedTestModel = getSuggestedTestModel(prev.protocolType);
      const nextSuggestedTestModel = getSuggestedTestModel(nextProtocolType);
      const prevSuggestedUpstreamModel = getSuggestedUpstreamModel(
        prev.protocolType,
      );
      const nextSuggestedUpstreamModel = getSuggestedUpstreamModel(
        nextProtocolType,
      );

      const shouldResetTestModel =
        prev.testModel.trim() === "" || prev.testModel === prevSuggestedTestModel;
      const shouldResetUpstreamModel =
        prev.upstreamModel.trim() === "" ||
        prev.upstreamModel === prevSuggestedUpstreamModel;

      return {
        ...prev,
        protocolType: nextProtocolType,
        testModel: shouldResetTestModel
          ? nextSuggestedTestModel
          : prev.testModel,
        upstreamModel:
          nextProtocolType === "anthropic"
            ? ""
            : shouldResetUpstreamModel
              ? nextSuggestedUpstreamModel
              : prev.upstreamModel,
      };
    });
  };

  const updateField = (field: keyof ProviderFormData, value: string) => {
    setForm((prev) => ({ ...prev, [field]: value }));
    if (errors[field]) {
      setErrors((prev) => {
        const next = { ...prev };
        delete next[field];
        return next;
      });
    }
  };

  const addModelMapEntry = () => {
    setForm((prev) => ({
      ...prev,
      upstreamModelMap: [...prev.upstreamModelMap, { source: "", target: "" }],
    }));
  };

  const removeModelMapEntry = (idx: number) => {
    setForm((prev) => ({
      ...prev,
      upstreamModelMap: prev.upstreamModelMap.filter((_, i) => i !== idx),
    }));
  };

  const updateModelMapEntry = (
    idx: number,
    field: keyof ModelMapEntry,
    value: string,
  ) => {
    setForm((prev) => ({
      ...prev,
      upstreamModelMap: prev.upstreamModelMap.map((entry, i) =>
        i === idx ? { ...entry, [field]: value } : entry,
      ),
    }));
  };

  const showModelMapping =
    form.protocolType === "open_ai_chat_completions" ||
    form.protocolType === "open_ai_responses";

  const handleSave = async () => {
    const result = formSchema.safeParse({
      name: form.name,
      apiKey: form.apiKey,
      baseUrl: form.baseUrl,
    });

    if (!result.success) {
      const fieldErrors: Record<string, string> = {};
      for (const issue of result.error.issues) {
        const field = issue.path[0] as string;
        if (field === "name") fieldErrors.name = t("validation.nameRequired");
        if (field === "apiKey")
          fieldErrors.apiKey = t("validation.apiKeyRequired");
        if (field === "baseUrl")
          fieldErrors.baseUrl = t("validation.baseUrlRequired");
      }
      setErrors(fieldErrors);
      return;
    }

    if (showModelMapping && form.upstreamModel.trim().length === 0) {
      setErrors((prev) => ({
        ...prev,
        upstreamModel: t("validation.upstreamModelRequired"),
      }));
      return;
    }

    setSaving(true);
    try {
      await onSave(form);
      onOpenChange(false);
    } catch (err) {
      console.error("Save failed:", err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[640px] flex flex-col max-h-[85vh]">
        {/* 固定 Header */}
        <DialogHeader className="flex-shrink-0">
          <DialogTitle>
            {mode === "create"
              ? t("dialog.createTitle")
              : t("dialog.editTitle")}
          </DialogTitle>
        </DialogHeader>

        {/* 中间表单区域 — 独立滚动 */}
        <div className="overflow-y-auto flex-1 min-h-0">
          <div className="flex flex-col gap-4 px-1">

            {/* 分区 1 — 基础信息 */}
            <div className="flex items-center gap-2">
              <span className="text-sm font-semibold text-muted-foreground">{t("section.basic")}</span>
              <div className="flex-1 border-t border-border" />
            </div>

            {/* Name */}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="provider-name">{t("provider.name")}</Label>
              <Input
                id="provider-name"
                value={form.name}
                onChange={(e) => updateField("name", e.target.value)}
                placeholder={t("placeholder.name")}
                aria-invalid={!!errors.name}
              />
              {errors.name && (
                <p className="text-xs text-destructive">{errors.name}</p>
              )}
            </div>

            {/* API Key */}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="provider-api-key">{t("provider.apiKey")}</Label>
              <div className="relative">
                <Input
                  id="provider-api-key"
                  type={showApiKey ? "text" : "password"}
                  value={form.apiKey}
                  onChange={(e) => updateField("apiKey", e.target.value)}
                  placeholder={t("placeholder.apiKey")}
                  className="pr-10"
                  aria-invalid={!!errors.apiKey}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  className="absolute right-2 top-1/2 -translate-y-1/2"
                  onClick={() => setShowApiKey(!showApiKey)}
                >
                  {showApiKey ? (
                    <EyeOff className="size-3.5" />
                  ) : (
                    <Eye className="size-3.5" />
                  )}
                </Button>
              </div>
              {errors.apiKey && (
                <p className="text-xs text-destructive">{errors.apiKey}</p>
              )}
            </div>

            {/* Base URL */}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="provider-base-url">{t("provider.baseUrl")}</Label>
              <Input
                id="provider-base-url"
                value={form.baseUrl}
                onChange={(e) => updateField("baseUrl", e.target.value)}
                placeholder={t("placeholder.baseUrl")}
                aria-invalid={!!errors.baseUrl}
              />
              {errors.baseUrl && (
                <p className="text-xs text-destructive">{errors.baseUrl}</p>
              )}
            </div>

            {/* 分区 2 — 协议设置 */}
            <div className="flex items-center gap-2 pt-2">
              <span className="text-sm font-semibold text-muted-foreground">{t("section.protocol")}</span>
              <div className="flex-1 border-t border-border" />
            </div>

            {/* Protocol Type */}
            <div className="flex flex-col gap-1.5">
              <Label>{t("provider.protocolType")}</Label>
              <Select
                value={form.protocolType}
                onValueChange={(v) => updateProtocolType(v as ProtocolType)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="anthropic">
                    {t("protocol.anthropic")}
                  </SelectItem>
                  <SelectItem value="open_ai_chat_completions">
                    {t("protocol.openAiChatCompletions")}
                  </SelectItem>
                  <SelectItem value="open_ai_responses">
                    {t("protocol.openAiResponses")}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* 模型映射（仅 OpenAI 类型显示） */}
            {showModelMapping && (
              <>
                {/* 默认目标模型 */}
                <div className="flex flex-col gap-1.5">
                  <Label htmlFor="provider-upstream-model">
                    {t("provider.upstreamModel")}
                  </Label>
                  <Input
                    id="provider-upstream-model"
                    placeholder={getSuggestedUpstreamModel(form.protocolType)}
                    value={form.upstreamModel}
                    onChange={(e) =>
                      updateField("upstreamModel", e.target.value)
                    }
                    aria-invalid={!!errors.upstreamModel}
                  />
                  {errors.upstreamModel && (
                    <p className="text-xs text-destructive">
                      {errors.upstreamModel}
                    </p>
                  )}
                </div>

                {/* 模型名映射 */}
                <div className="flex flex-col gap-2">
                  <Label className="text-muted-foreground text-xs">
                    {t("provider.modelMapping")}
                  </Label>
                  {form.upstreamModelMap.map((entry, idx) => (
                    <div key={idx} className="flex items-center gap-2">
                      <Input
                        placeholder={t("provider.sourceModel")}
                        value={entry.source}
                        onChange={(e) =>
                          updateModelMapEntry(idx, "source", e.target.value)
                        }
                        className="flex-1"
                      />
                      <Input
                        placeholder={t("provider.targetModel")}
                        value={entry.target}
                        onChange={(e) =>
                          updateModelMapEntry(idx, "target", e.target.value)
                        }
                        className="flex-1"
                      />
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-xs"
                        onClick={() => removeModelMapEntry(idx)}
                      >
                        <X className="size-3.5" />
                      </Button>
                    </div>
                  ))}
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="w-full"
                    onClick={addModelMapEntry}
                  >
                    {t("provider.addMapping")}
                  </Button>
                </div>
              </>
            )}

            {/* 分区 3 — 模型配置 */}
            <div className="flex items-center gap-2 pt-2">
              <span className="text-sm font-semibold text-muted-foreground">{t("section.model")}</span>
              <div className="flex-1 border-t border-border" />
            </div>

            {/* Model */}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="provider-model">{t("provider.model")}</Label>
              <Input
                id="provider-model"
                value={form.model}
                onChange={(e) => updateField("model", e.target.value)}
                placeholder={t("placeholder.model")}
              />
            </div>

            {/* Test Model */}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="provider-test-model">
                {t("provider.testModel")}
              </Label>
              <Input
                id="provider-test-model"
                placeholder={getSuggestedTestModel(form.protocolType)}
                value={form.testModel}
                onChange={(e) => updateField("testModel", e.target.value)}
              />
            </div>

            {/* Model Config — 2x2 grid */}
            <div className="grid grid-cols-2 gap-2">
              <Input
                placeholder={t("placeholder.haikuModel")}
                value={form.haikuModel}
                onChange={(e) => updateField("haikuModel", e.target.value)}
              />
              <Input
                placeholder={t("placeholder.sonnetModel")}
                value={form.sonnetModel}
                onChange={(e) => updateField("sonnetModel", e.target.value)}
              />
              <Input
                placeholder={t("placeholder.opusModel")}
                value={form.opusModel}
                onChange={(e) => updateField("opusModel", e.target.value)}
              />
              <Input
                placeholder={t("placeholder.reasoningEffort")}
                value={form.reasoningEffort}
                onChange={(e) =>
                  updateField("reasoningEffort", e.target.value)
                }
              />
            </div>

            {/* Notes */}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="provider-notes">{t("provider.notes")}</Label>
              <Input
                id="provider-notes"
                value={form.notes}
                onChange={(e) => updateField("notes", e.target.value)}
                placeholder={t("placeholder.notes")}
              />
            </div>

          </div>
        </div>

        {/* 固定 Footer */}
        <DialogFooter className="flex-shrink-0 border-t pt-4">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("actions.cancel")}
          </Button>
          <Button onClick={handleSave} disabled={saving}>
            {saving && <Loader2 className="size-4 animate-spin" />}
            {t("actions.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
