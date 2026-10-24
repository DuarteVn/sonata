import { Gauge } from "lucide-react";
import { useEffect } from "react";
import { UsageConsentDialog } from "../dialogs/UsageConsentDialog";
import { onEvent } from "../ipc/events";
import { useUsage } from "../state/usage";
import { UsageSettings } from "./UsageSettings";

// Root of the Settings window (`?view=settings`, FR-015). Usage is the only section for now.
export function SettingsApp() {
  const consent = useUsage((s) => s.consents[0]);

  useEffect(() => {
    void useUsage.getState().load();
    const unlisten = [
      onEvent("usage://state", (state) => useUsage.getState().set(state)),
      // The main window shows consent requests unless this window has focus.
      onEvent("usage://consent", (request) => {
        if (document.hasFocus()) useUsage.getState().askConsent(request);
      }),
    ];
    return () => unlisten.forEach((p) => p.then((off) => off()));
  }, []);

  return (
    <div className="settings-app">
      <nav className="settings-nav" aria-label="Settings sections">
        <button type="button" className="settings-nav-item active" aria-current="page">
          <Gauge size={14} /> Usage
        </button>
      </nav>
      <main className="settings-content">
        <UsageSettings />
      </main>
      {consent && (
        <UsageConsentDialog
          key={consent.providerId}
          request={consent}
          onDone={() => useUsage.getState().dropConsent(consent.providerId)}
        />
      )}
    </div>
  );
}
