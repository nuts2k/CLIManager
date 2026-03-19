import { useTranslation } from "react-i18next";
import { Activity } from "lucide-react";

export function TrafficEmptyState() {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col items-center justify-center gap-3 py-20 text-center">
      <Activity className="size-10 text-muted-foreground/50" />
      <div>
        <h3 className="text-base font-medium">{t("traffic.empty.title")}</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          {t("traffic.empty.description")}
        </p>
      </div>
    </div>
  );
}
