import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Eye, EyeOff, Loader2, ChevronDown, X } from "lucide-react";
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
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
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
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  const [form, setForm] = useState<ProviderFormData>({
    name: "",
    apiKey: "",
    baseUrl: "",
    model: "",
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
      setAdvancedOpen(false);
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
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {mode === "create"
              ? t("dialog.createTitle")
              : t("dialog.editTitle")}
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          {/* Name */}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="provider-name">{t("provider.name")}</Label>
            <Input
              id="provider-name"
              value={form.name}
              onChange={(e) => updateField("name", e.target.value)}
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
              aria-invalid={!!errors.baseUrl}
            />
            {errors.baseUrl && (
              <p className="text-xs text-destructive">{errors.baseUrl}</p>
            )}
          </div>

          {/* Advanced section */}
          <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
            <CollapsibleTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="w-full justify-between"
              >
                {t("provider.advanced")}
                <ChevronDown
                  className={`size-4 transition-transform ${advancedOpen ? "rotate-180" : ""}`}
                />
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="flex flex-col gap-4 pt-2">
              {/* Model */}
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="provider-model">{t("provider.model")}</Label>
                <Input
                  id="provider-model"
                  value={form.model}
                  onChange={(e) => updateField("model", e.target.value)}
                />
              </div>

              {/* Protocol Type */}
              <div className="flex flex-col gap-1.5">
                <Label>{t("provider.protocolType")}</Label>
                <Select
                  value={form.protocolType}
                  onValueChange={(v) =>
                    updateField("protocolType", v as ProtocolType)
                  }
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
                      placeholder="gpt-4o"
                      value={form.upstreamModel}
                      onChange={(e) =>
                        updateField("upstreamModel", e.target.value)
                      }
                    />
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

              {/* Notes */}
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="provider-notes">{t("provider.notes")}</Label>
                <Input
                  id="provider-notes"
                  value={form.notes}
                  onChange={(e) => updateField("notes", e.target.value)}
                />
              </div>

              {/* Model Config */}
              <div className="flex flex-col gap-2">
                <Label className="text-muted-foreground text-xs">
                  {t("provider.modelConfig")}
                </Label>
                <div className="grid grid-cols-2 gap-2">
                  <Input
                    placeholder="haiku_model"
                    value={form.haikuModel}
                    onChange={(e) => updateField("haikuModel", e.target.value)}
                  />
                  <Input
                    placeholder="sonnet_model"
                    value={form.sonnetModel}
                    onChange={(e) => updateField("sonnetModel", e.target.value)}
                  />
                  <Input
                    placeholder="opus_model"
                    value={form.opusModel}
                    onChange={(e) => updateField("opusModel", e.target.value)}
                  />
                  <Input
                    placeholder="reasoning_effort"
                    value={form.reasoningEffort}
                    onChange={(e) =>
                      updateField("reasoningEffort", e.target.value)
                    }
                  />
                </div>
              </div>
            </CollapsibleContent>
          </Collapsible>
        </div>

        <DialogFooter>
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
