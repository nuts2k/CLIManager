import { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSettings } from "@/hooks/useSettings";
import i18n from "@/i18n";

interface SettingsPageProps {
  onBack: () => void;
  onShowImport?: () => void;
}

export function SettingsPage({ onBack, onShowImport }: SettingsPageProps) {
  const { t } = useTranslation();
  const { settings, updateSettings } = useSettings();

  const currentLanguage = settings?.language ?? "zh";

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

      <div className="flex-1 overflow-auto p-6 space-y-6">
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

        <Separator />

        {/* About Section */}
        <section className="space-y-3">
          <h3 className="text-sm font-medium text-foreground">
            {t("settings.about")}
          </h3>
          <div className="space-y-1 text-sm text-muted-foreground">
            <p>
              {t("settings.version")}: 0.1.0
            </p>
            <p>CLIManager - CLI Provider Management Tool</p>
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
      </div>
    </div>
  );
}
