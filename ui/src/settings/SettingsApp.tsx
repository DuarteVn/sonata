import { Bot, Gauge } from "lucide-react";
import { useEffect, useState } from "react";
import { UsageConsentDialog } from "../dialogs/UsageConsentDialog";
import { onEvent } from "../ipc/events";
import { useUsage } from "../state/usage";
import { AgentSettings } from "./AgentSettings";
import { UsageSettings } from "./UsageSettings";

type Section = "usage" | "agents";

function initialSection(): Section {
  return (window as { __SONATA_SECTION__?: string }).__SONATA_SECTION__ === "agents" ? "agents" : "usage";
}

// Root of the Settings window (`?view=settings`, FR-015): Usage (006) and Agents (010).
export function SettingsApp() {
  const consent = useUsage((s) => s.consents[0]);
  const [section, setSection] = useState<Section>(initialSection);

  useEffect(() => {
    void useUsage.getState().load();
    const unlisten = [
      onEvent("usage://state", (state) => useUsage.getState().set(state)),
      // The main window shows consent requests unless this window has focus.
      onEvent("usage://consent", (request) => {
        if (document.hasFocus()) useUsage.getState().askConsent(request);
      }),
      onEvent("settings://section", (s) => setSection(s === "agents" ? "agents" : "usage")),
    ];
    return () => unlisten.forEach((p) => p.then((off) => off()));
  }, []);

  return (
    <div className="settings-app">
      <nav className="settings-nav" aria-label="Settings sections">
        {(
          [
            ["usage", "Usage", Gauge],
            ["agents", "Agents", Bot],
          ] as const
        ).map(([id, label, Icon]) => (
          <button
            key={id}
            type="button"
            className={`settings-nav-item${section === id ? " active" : ""}`}
            aria-current={section === id ? "page" : undefined}
            onClick={() => setSection(id)}
          >
            <Icon size={14} /> {label}
          </button>
        ))}
      </nav>
      <main className="settings-content">{section === "usage" ? <UsageSettings /> : <AgentSettings />}</main>
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
