import { Plus, PackageOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

interface EmptyStateProps {
  onCreate: () => void;
}

export function EmptyState({ onCreate }: EmptyStateProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col items-center justify-center gap-4 py-16">
      <PackageOpen className="size-12 text-muted-foreground" />
      <div className="text-center">
        <h3 className="text-lg font-medium">{t("empty.title")}</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          {t("empty.description")}
        </p>
      </div>
      <Button onClick={onCreate}>
        <Plus className="size-4" />
        {t("actions.create")}
      </Button>
    </div>
  );
}
