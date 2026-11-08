import { useState } from "react";
import { AlertTriangle, ChevronLeft, ChevronRight, LockKeyhole, Plug, Shield, ShieldAlert, ShieldCheck } from "lucide-react";

const providers = [
  { id: "wsc", name: "Windows Security Center", os: "Windows", detail: "Read registered antivirus and firewall provider status." },
  { id: "defender", name: "Microsoft Defender", os: "Windows", detail: "Read protection state, update freshness and supported threat summaries." },
  { id: "win-firewall", name: "Windows Firewall", os: "Windows", detail: "Read firewall profile status; never change rules." },
  { id: "clamav", name: "ClamAV", os: "Linux", detail: "Optional local service status and signature freshness." },
  { id: "linux-firewall", name: "firewalld / nftables / UFW", os: "Linux", detail: "Optional read-only firewall service status." },
  { id: "macos", name: "macOS / vendor APIs", os: "macOS", detail: "Use documented native or vendor interfaces only." }
];

export default function SecurityCentre() {
  const [step, setStep] = useState(0);
  const [selected, setSelected] = useState<string[]>([]);
  const [consent, setConsent] = useState(false);
  const chosen = providers.filter((p) => selected.includes(p.id));
  const toggle = (id: string) => setSelected((old) => old.includes(id) ? old.filter((x) => x !== id) : [...old, id]);
  const reset = () => { setStep(0); setSelected([]); setConsent(false); };
  return <section className="security-centre" id="security-centre" tabIndex={-1} aria-labelledby="security-title">
    <div className="section-heading"><div><h2 id="security-title"><Shield size={19}/> Security Centre</h2><p>Review the security tools already installed on this device.</p></div><span className="security-prototype-label">EARLY UI PREVIEW</span></div>
    <div className="security-disclaimer" role="note"><LockKeyhole size={17}/><span><b>UJANI coordinates; it is not an antivirus or firewall.</b> This is a frontend prototype. Live provider discovery and adapters are not installed yet. It does not scan, block traffic, change settings, quarantine files or remediate threats.</span></div>
    <div className="security-metrics">
      <article className="security-metric"><ShieldCheck/><small>ANTIVIRUS / ENDPOINT</small><b>Unknown</b><span>No live provider data</span></article>
      <article className="security-metric"><Shield/><small>FIREWALL</small><b>Unknown</b><span>No live provider data</span></article>
      <article className="security-metric"><ShieldAlert/><small>THREAT ALERTS</small><b>Unavailable</b><span>No provider connected</span></article>
      <article className="security-metric"><AlertTriangle/><small>UPDATES & SCANS</small><b>Unknown</b><span>Freshness not verified</span></article>
    </div>
    <div className="security-workspace">
      <div className="security-panel"><h3>Connected providers</h3><p>Nothing is selected or connected by default.</p>
        <div className="security-empty"><b>No live security providers connected</b><small>Choose the wizard to review supported options. This preview does not perform operating-system discovery.</small></div>
        <button type="button" className="model-run" onClick={() => setStep(1)}><Plug size={15}/> Open connection wizard <ChevronRight size={15}/></button>
        {providers.map((p) => <div className="security-provider" key={p.id}><b>{p.name}</b><small>{p.os} · Not connected</small><p>{p.detail}</p></div>)}
      </div>
      <div className="security-panel"><h3>Privacy and permissions</h3>
        <p><b>Local-only by default</b><br/>Provider status stays on this device unless you explicitly enable Enterprise sharing.</p>
        <p><b>Read-only first</b><br/>No firewall changes, quarantine, process termination or automatic remediation.</p>
        <p><b>Honest status</b><br/>Missing, stale, unsupported or conflicting information is never shown as protected.</p>
      </div>
    </div>
    {step > 0 && <div className="security-wizard" role="region" aria-labelledby="security-wizard-title">
      <div className="section-heading"><div><h3 id="security-wizard-title">Connect a security provider</h3><p>Step {step} of 4</p></div><button type="button" className="security-secondary" onClick={reset}>Close</button></div>
      <ol className="security-steps"><li className={step===1?"current":""}>1. Choose</li><li className={step===2?"current":""}>2. Review data</li><li className={step===3?"current":""}>3. Permission</li><li className={step===4?"current":""}>4. Result</li></ol>
      {step===1 && <div className="security-step-content"><p>Select providers to review. Nothing is preselected or contacted in this preview.</p>{providers.map((p)=><label className="security-choice" key={p.id}><input type="checkbox" checked={selected.includes(p.id)} onChange={()=>toggle(p.id)}/><span><b>{p.name}</b><small>{p.os} · {p.detail}</small></span><em>Not connected</em></label>)}</div>}
      {step===2 && <div className="security-step-content"><p>Proposed metadata scope only. No file contents, packet payloads, credentials or command lines.</p>{chosen.map((p)=><article className="security-scope" key={p.id}><b>{p.name}</b><ul><li>Provider identity and registration state</li><li>Protection health and update freshness</li><li>Bounded provider-reported alert summary</li></ul></article>)}<p className="security-muted">Low-frequency/event-driven checks, bounded results, timeouts and backoff. No continuous AI inference.</p></div>}
      {step===3 && <div className="security-step-content"><p>Proposed permission: local read-only status metadata. No changes to your security software or operating system.</p><label className="security-consent-check"><input type="checkbox" checked={consent} onChange={(e)=>setConsent(e.target.checked)}/> I understand the proposed data scope and want to preview the result.</label></div>}
      {step===4 && <div className="security-step-content"><div className="security-result"><AlertTriangle/><span><b>No connection was made</b><p>Native discovery and provider adapters still need implementation and platform testing. No access was granted and no settings were changed.</p></span></div><p className="security-muted">Selected providers: {chosen.map((p)=>p.name).join(", ") || "None"}.</p><button type="button" className="security-secondary" onClick={reset}>Start again</button></div>}
      <div className="security-wizard-actions"><button type="button" className="security-secondary" onClick={()=>setStep((s)=>Math.max(1,s-1))} disabled={step===1}><ChevronLeft size={15}/> Back</button>{step<3 && <button type="button" className="model-run" onClick={()=>setStep((s)=>s+1)} disabled={step===1 && selected.length===0}>Continue <ChevronRight size={15}/></button>}{step===3 && <button type="button" className="model-run" onClick={()=>setStep(4)} disabled={!consent}>Preview result <ChevronRight size={15}/></button>}</div>
    </div>}
  </section>;
}
