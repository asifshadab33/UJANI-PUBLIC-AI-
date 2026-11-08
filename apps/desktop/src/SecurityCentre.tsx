import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, ChevronLeft, ChevronRight, LockKeyhole, Plug, RefreshCw, Shield, ShieldAlert, ShieldCheck } from "lucide-react";

type ProviderStatus = {
  provider_id: string;
  label: string;
  status: string;
  source: string;
  detail: string;
  observed_at_unix_ms: number;
};

const providers = [
  { id: "windows.antivirus", name: "Windows Security Center — antivirus category", os: "Windows", detail: "Read aggregate antivirus category health. Does not identify the installed product." },
  { id: "windows.firewall", name: "Windows Security Center — firewall category", os: "Windows", detail: "Read aggregate firewall category health. Does not inspect or change firewall rules." }
];

const plannedProviders = [
  { name: "Microsoft Defender details", os: "Windows", detail: "A separate documented Defender status adapter is not implemented yet." },
  { name: "ClamAV / Linux firewall", os: "Linux", detail: "Optional local provider adapters are planned; no Linux service is queried in this build." },
  { name: "macOS / vendor security APIs", os: "macOS", detail: "Supported documented APIs must be implemented and tested before status can be reported." }
];

function statusLabel(status?: string) {
  switch (status) {
    case "healthy": return "Healthy";
    case "attention": return "Attention";
    case "alert": return "Alert";
    case "not_monitored": return "Not monitored";
    case "unavailable": return "Unavailable";
    case "unsupported": return "Unsupported";
    default: return "Unknown";
  }
}

function statusDetail(status?: ProviderStatus) {
  if (!status) return "No status check has been performed";
  return status.detail;
}

export default function SecurityCentre() {
  const [step, setStep] = useState(0);
  const [selected, setSelected] = useState<string[]>([]);
  const [consent, setConsent] = useState(false);
  const [localChecksEnabled, setLocalChecksEnabled] = useState(false);
  const [statuses, setStatuses] = useState<ProviderStatus[]>([]);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState("");
  const chosen = providers.filter((p) => selected.includes(p.id));
  const antivirus = statuses.find((p) => p.provider_id === "windows.antivirus");
  const firewall = statuses.find((p) => p.provider_id === "windows.firewall");

  const toggle = (id: string) => setSelected((old) => old.includes(id) ? old.filter((x) => x !== id) : [...old, id]);
  const reset = () => { setStep(0); setSelected([]); setConsent(false); setError(""); };

  const refreshStatus = async () => {
    setChecking(true);
    setError("");
    try {
      const result = await invoke<ProviderStatus[]>("get_security_provider_status");
      setStatuses(result);
      setLocalChecksEnabled(true);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "The local status check failed. No system settings were changed.");
    } finally {
      setChecking(false);
    }
  };

  const enableAndCheck = async () => {
    await refreshStatus();
    setStep(4);
  };

  return <section className="security-centre" id="security-centre" tabIndex={-1} aria-labelledby="security-title">
    <div className="section-heading"><div><h2 id="security-title"><Shield size={19}/> Security Centre</h2><p>Review supported security-provider status already available on this device.</p></div><span className="security-prototype-label">READ-ONLY PREVIEW</span></div>
    <div className="security-disclaimer" role="note"><LockKeyhole size={17}/><span><b>UJANI coordinates; it is not an antivirus or firewall.</b> The first native adapter reads Windows Security Center's aggregate antivirus and firewall health only. It does not identify individual products, inspect files, collect threat events, change settings, block traffic, quarantine files or remediate threats. Linux and macOS adapters are not implemented yet.</span></div>
    <div className="security-metrics">
      <article className="security-metric"><ShieldCheck/><small>ANTIVIRUS CATEGORY</small><b>{statusLabel(antivirus?.status)}</b><span>{statusDetail(antivirus)}</span></article>
      <article className="security-metric"><Shield/><small>FIREWALL CATEGORY</small><b>{statusLabel(firewall?.status)}</b><span>{statusDetail(firewall)}</span></article>
      <article className="security-metric"><ShieldAlert/><small>THREAT ALERTS</small><b>Not collected</b><span>No threat-event feed is connected</span></article>
      <article className="security-metric"><AlertTriangle/><small>UPDATES & SCANS</small><b>Unknown</b><span>Update and scan freshness are not provided by this aggregate API</span></article>
    </div>
    <div className="security-workspace">
      <div className="security-panel"><h3>Local provider status</h3><p>Nothing is queried until you explicitly enable a local status check.</p>
        {!localChecksEnabled && <div className="security-empty"><b>No local status check performed</b><small>Choose the wizard to review the exact metadata and approve a one-time read-only check.</small></div>}
        {localChecksEnabled && <div className="security-empty"><b>Local status check enabled for this session</b><small>Results remain on this device. This permission is not persisted after the application session ends.</small></div>}
        {localChecksEnabled && <button type="button" className="model-run" onClick={refreshStatus} disabled={checking}><RefreshCw size={15}/> {checking ? "Checking…" : "Refresh local status"}</button>}
        {!localChecksEnabled && <button type="button" className="model-run" onClick={() => setStep(1)}><Plug size={15}/> Review local status checks <ChevronRight size={15}/></button>}
        {providers.map((p) => {
          const result = statuses.find((item) => item.provider_id === p.id);
          return <div className="security-provider" key={p.id}><b>{p.name}</b><small>{p.os} · {statusLabel(result?.status)}</small><p>{result?.detail ?? p.detail}</p>{result && <small>Source: {result.source}</small>}</div>;
        })}
        {plannedProviders.map((p) => <div className="security-provider" key={p.name}><b>{p.name}</b><small>{p.os} · Planned, not connected</small><p>{p.detail}</p></div>)}
        {statuses.length > 0 && <p className="security-muted">Last checked: {new Date(Math.max(...statuses.map((s) => s.observed_at_unix_ms))).toLocaleString()}</p>}
        {error && <p className="security-error" role="alert">{error}</p>}
      </div>
      <div className="security-panel"><h3>Privacy and permissions</h3>
        <p><b>Local-only by default</b><br/>The Windows API result is returned to this application and is not uploaded or shared.</p>
        <p><b>Read-only first</b><br/>No firewall changes, quarantine, process termination or automatic remediation.</p>
        <p><b>Honest status</b><br/>Missing, stale, unsupported or conflicting information is never shown as protected.</p>
        <p><b>Aggregate status only</b><br/>Windows Security Center reports category health, not the name or full capabilities of each installed security product.</p>
      </div>
    </div>
    {step > 0 && <div className="security-wizard" role="region" aria-labelledby="security-wizard-title">
      <div className="section-heading"><div><h3 id="security-wizard-title">Enable local read-only status checks</h3><p>Step {Math.min(step, 4)} of 4</p></div><button type="button" className="security-secondary" onClick={reset}>Close</button></div>
      <ol className="security-steps"><li className={step===1?"current":""}>1. Choose</li><li className={step===2?"current":""}>2. Review data</li><li className={step===3?"current":""}>3. Permission</li><li className={step===4?"current":""}>4. Result</li></ol>
      {step===1 && <div className="security-step-content"><p>Select the local status categories you want UJANI to query. Nothing is preselected.</p>{providers.map((p)=><label className="security-choice" key={p.id}><input type="checkbox" checked={selected.includes(p.id)} onChange={()=>toggle(p.id)}/><span><b>{p.name}</b><small>{p.os} · {p.detail}</small></span><em>Read-only</em></label>)}<p className="security-muted">Other provider integrations are listed as planned above and cannot be enabled yet.</p></div>}
      {step===2 && <div className="security-step-content"><p>The one-time query reads only the aggregate health value for the selected Windows Security Center categories.</p>{chosen.map((p)=><article className="security-scope" key={p.id}><b>{p.name}</b><ul><li>Aggregate category health (healthy, not monitored, poor, snoozed or unknown)</li><li>Local observation timestamp</li><li>API source and non-sensitive status explanation</li></ul></article>)}<p className="security-muted">No product names, file paths, file contents, packet payloads, credentials, command lines or threat-event records are collected. No network request or continuous polling is performed.</p></div>}
      {step===3 && <div className="security-step-content"><div className="security-consent"><LockKeyhole/><div><b>One-time local read-only check</b><p>UJANI will call the supported Windows Security Center status API for the selected categories. This does not change operating-system or security-product settings. Consent lasts only for this application session.</p></div></div><label className="security-consent-check"><input type="checkbox" checked={consent} onChange={(e)=>setConsent(e.target.checked)}/> I approve this local read-only status check and understand its limited scope.</label></div>}
      {step===4 && <div className="security-step-content">{error ? <div className="security-result"><AlertTriangle/><span><b>Status check could not be completed</b><p>{error}</p></span></div> : <div className="security-result"><ShieldCheck/><span><b>Local status query completed</b><p>No security settings were changed. Review the category status and source in the Security Centre overview. An unsupported or unavailable result is not proof of protection.</p></span></div>}<p className="security-muted">Selected categories: {chosen.map((p)=>p.name).join(", ") || "None"}.</p><button type="button" className="security-secondary" onClick={reset}>Close wizard</button></div>}
      <div className="security-wizard-actions"><button type="button" className="security-secondary" onClick={()=>setStep((s)=>Math.max(1,s-1))} disabled={step===1 || checking}><ChevronLeft size={15}/> Back</button>{step<3 && <button type="button" className="model-run" onClick={()=>setStep((s)=>s+1)} disabled={(step===1 && selected.length===0) || checking}>Continue <ChevronRight size={15}/></button>}{step===3 && <button type="button" className="model-run" onClick={enableAndCheck} disabled={!consent || checking}>{checking ? "Checking…" : "Approve & check locally"} <ChevronRight size={15}/></button>}</div>
    </div>}
  </section>;
}
