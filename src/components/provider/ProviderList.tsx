import { ScrollArea } from "@/components/ui/scroll-area";
import { ProviderCard } from "@/components/provider/ProviderCard";
import { EmptyState } from "@/components/provider/EmptyState";
import type { Provider } from "@/types/provider";

interface ProviderListProps {
  providers: Provider[];
  activeProviderId: string | null;
  loading: boolean;
  currentCliId: string;
  operationLoading: string | null;
  onCreate: () => void;
  onSwitch: (providerId: string) => void;
  onEdit: (provider: Provider) => void;
  onCopy: (provider: Provider) => void;
  onCopyTo: (provider: Provider, targetCliId: string) => void;
  onTest: (providerId: string) => void;
  onDelete: (provider: Provider) => void;
}

export function ProviderList({
  providers,
  activeProviderId,
  loading,
  currentCliId,
  operationLoading,
  onCreate,
  onSwitch,
  onEdit,
  onCopy,
  onCopyTo,
  onTest,
  onDelete,
}: ProviderListProps) {
  if (!loading && providers.length === 0) {
    return <EmptyState onCreate={onCreate} />;
  }

  return (
    <ScrollArea className="h-[calc(100vh-10rem)]">
      <div className="flex flex-col gap-2 p-1">
        {providers.map((provider) => (
          <ProviderCard
            key={provider.id}
            provider={provider}
            isActive={provider.id === activeProviderId}
            currentCliId={currentCliId}
            operationLoading={operationLoading}
            onSwitch={() => onSwitch(provider.id)}
            onEdit={() => onEdit(provider)}
            onCopy={() => onCopy(provider)}
            onCopyTo={(targetCliId) => onCopyTo(provider, targetCliId)}
            onTest={() => onTest(provider.id)}
            onDelete={() => onDelete(provider)}
          />
        ))}
      </div>
    </ScrollArea>
  );
}
