import { AppShell } from "@/components/layout/AppShell";
import { Toaster } from "@/components/ui/sonner";

function App() {
  return (
    <>
      <AppShell />
      <Toaster position="bottom-right" theme="dark" />
    </>
  );
}

export default App;
