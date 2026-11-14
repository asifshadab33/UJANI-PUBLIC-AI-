type FeatureTone = "available" | "opt-in" | "preview" | "blocked" | "planned";

type FeatureEntry = {
  id: string;
  title: string;
  status: string;
  tone: FeatureTone;
  risk: string;
  summary: string;
  permissions: string;
  data: string;
  limits: string;
};

const featureEntries: FeatureEntry[] = [
  {
    id: "local-search",
    title: "Local file and folder search",
    status: "Available · user initiated",
    tone: "available",
    risk: "Medium",
    summary: "Search filenames and supported text content inside a folder you select.",
    permissions: "A native folder picker and the selected folder only. No automatic whole-disk scan; it operates only within the user-selected scope and configured bounds, with no background indexer.",
    data: "Processed locally. This feature does not upload the selected files to a cloud provider.",
    limits: "Bounded to 5,000 entries, six directory levels, 2 MiB per text file, 20 text-based PDFs and a 10-minute search budget. Scanned PDFs need OCR, which is not included.",
  },
  {
    id: "folder-import",
    title: "Selected-folder document import",
    status: "Available · bounded",
    tone: "available",
    risk: "Medium",
    summary: "Import a small, user-selected set of supported files for local document processing.",
    permissions: "The user chooses the folder through the native picker. The operation is read-only and can be cancelled.",
    data: "Local processing by default. No silent upload, move or deletion.",
    limits: "Bounded file count, per-file size, aggregate bytes and scan duration. Symbolic links and unsupported files are excluded.",
  },
  {
    id: "pdf-text",
    title: "PDF text processing",
    status: "Available · text PDFs",
    tone: "available",
    risk: "Medium",
    summary: "Extract and search embedded text in supported PDF documents.",
    permissions: "Requires an explicit file or folder selection.",
    data: "Local PDF worker and local search path. External AI is a separate task requiring separate consent.",
    limits: "Bounded PDF count, per-file and aggregate size, worker time and cancellation. OCR for scanned/image-only PDFs is not included.",
  },
  {
    id: "image-inspection",
    title: "Image inspection",
    status: "Basic header inspection only",
    tone: "preview",
    risk: "Low",
    summary: "Read supported image headers and basic dimensions; this is not visual understanding.",
    permissions: "User supplies the image or selects it through a supported local workflow.",
    data: "Header bytes are inspected locally; no image is uploaded by this operation.",
    limits: "PNG, JPEG, GIF, WebP and BMP header inspection only. No OCR, image captioning or general vision model is enabled.",
  },
  {
    id: "video-discovery",
    title: "Video discovery",
    status: "Filename discovery only",
    tone: "preview",
    risk: "Low",
    summary: "Locate video files by supported filename and folder search.",
    permissions: "User-selected folder scope.",
    data: "Local filename and supported metadata search.",
    limits: "No frame extraction, transcription, audio understanding or semantic video search is enabled.",
  },
  {
    id: "search-cache",
    title: "Encrypted search-result cache",
    status: "Implemented · opt-in",
    tone: "opt-in",
    risk: "Medium",
    summary: "Remember validated search metadata and recent query phrases to speed up repeated searches.",
    permissions: "The device owner must explicitly enable the Local Finder setting. It is off by default.",
    data: "Metadata and recent query phrases are encrypted using an OS credential-vault key. Document contents, OCR text and embeddings are not cached.",
    limits: "7, 30 or 90 days, or until cleared; maximum 50 entries / 10 MiB. No-result entries expire after 24 hours. Disabling caching clears stored entries.",
  },
  {
    id: "local-models",
    title: "Local model selection",
    status: "Manual setup only · managed pull blocked",
    tone: "opt-in",
    risk: "Medium",
    summary: "Choose a model running through the local Ollama runtime; a fresh installation does not silently select one.",
    permissions: "The device owner selects the model and may install it directly through a trusted Ollama interface. UJANI-managed downloads remain disabled until exact artifact, licence, size, model-store path and free-space checks are available.",
    data: "Local inference stays on the device. A separate external-provider request requires per-task consent.",
    limits: "No automatic installation or UJANI-managed model pull is enabled. The catalogue is advisory until artifact and storage preflight is implemented.",
  },
  {
    id: "external-ai",
    title: "External AI / BYOK",
    status: "Per-task approval required",
    tone: "opt-in",
    risk: "High",
    summary: "Use a user-configured external provider only when the user chooses an allowed mode and approves the specific transfer.",
    permissions: "Provider configuration and explicit task-level consent. Fortress remains local-only.",
    data: "Only the approved task payload may leave the device. UJANI must not silently switch local data to cloud processing.",
    limits: "Provider availability, endpoint validation, payload bounds and policy checks remain mandatory. This is not permission for background uploads.",
  },
  {
    id: "ocr-audio",
    title: "OCR and audio transcription",
    status: "Not implemented · separate runtime required",
    tone: "planned",
    risk: "Medium",
    summary: "OCR and speech-to-text candidates are listed for future integration, but no production OCR or transcription runtime is enabled.",
    permissions: "Would require explicit file/audio selection, language-pack disclosure and bounded local processing.",
    data: "No OCR output, audio recording or transcript is currently collected by this capability entry.",
    limits: "Tesseract and Whisper are catalogue candidates only; exact runtime, artifact, licence, platform and resource requirements must be validated before activation.",
  },
  {
    id: "screenshot-capture",
    title: "Screenshot capture",
    status: "Not implemented",
    tone: "planned",
    risk: "High",
    summary: "UJANI does not currently provide a background screenshot or screen-monitoring capability.",
    permissions: "No screen-recording or screenshot permission is requested by this feature register.",
    data: "No screenshots or screen contents are collected.",
    limits: "Any future capture feature requires a separate explicit user action, visible capture state, narrow scope, retention controls and independent privacy review.",
  },
  {
    id: "planner",
    title: "Personal Intelligence and daily planner",
    status: "Design only · no runtime integration",
    tone: "planned",
    risk: "High",
    summary: "A future optional coordinator for user-selected calendar, reminder and task sources.",
    permissions: "No provider account is connected by this design. Future adapters require separate account, data-category and permission consent.",
    data: "No calendar, email or meeting data is read by this design specification.",
    limits: "No autonomous scheduling, message sending, calendar mutation or permanent agent loop is enabled.",
  },
  {
    id: "security-centre",
    title: "Security Centre",
    status: "Read-only preview",
    tone: "preview",
    risk: "High",
    summary: "Shows limited security-provider status and a connection-wizard preview.",
    permissions: "No firewall, antivirus, process or operating-system configuration changes are authorized by this preview.",
    data: "Provider status only where a supported native adapter is available.",
    limits: "Provider and platform coverage is limited. No remediation, quarantine, rule editing or automatic containment.",
  },
  {
    id: "peer-execution",
    title: "Worker / Partner peer execution",
    status: "Blocked pending safety gates",
    tone: "blocked",
    risk: "Critical",
    summary: "Delegating workloads to another device is not enabled.",
    permissions: "No administrator or user override. Device trust primitives do not grant remote execution authority.",
    data: "No remote task payloads or peer execution requests are sent.",
    limits: "Requires authenticated transport, receiver-side authorization, OS-enforced isolation, quotas, cancellation, revocation and independent security review.",
  },
  {
    id: "self-evolution",
    title: "Self-Evolution Engine",
    status: "Proposal-only development",
    tone: "planned",
    risk: "High",
    summary: "The first proposal contract is being developed; the installed app cannot yet rewrite itself.",
    permissions: "No runtime source access, code execution, self-approval, signing or automatic installation.",
    data: "A future source-level workflow requires a user-selected trusted checkout and explicit disclosure before any external model receives source context.",
    limits: "The protected core remains outside model authority. Signed releases require a separate trusted build and signing process.",
  },
];

export default function FeaturePermissionsCentre() {
  return (
    <section
      className="feature-permissions-centre"
      id="feature-permissions-centre"
      tabIndex={-1}
      aria-labelledby="feature-permissions-title"
    >
      <div className="section-heading">
        <div>
          <h2 id="feature-permissions-title">Feature &amp; Permissions Centre</h2>
          <p>See what is available, what requires your consent and what remains blocked.</p>
        </div>
        <span className="security-prototype-label">DEVICE OWNER VIEW</span>
      </div>
      <div className="feature-centre-disclaimer" role="note">
        <b>Read-only capability register.</b> This screen does not grant permissions or enable hidden functionality. A feature is configurable only when its native enforcement, consent and recovery controls are implemented and verified.
      </div>
      <div className="feature-centre-grid">
        {featureEntries.map((feature) => (
          <article className="feature-centre-card" key={feature.id}>
            <div className="feature-centre-card-heading">
              <h3>{feature.title}</h3>
              <span className={"feature-centre-status feature-centre-status--" + feature.tone}>{feature.status}</span>
            </div>
            <p className="feature-centre-summary">{feature.summary}</p>
            <div className="feature-centre-risk">Risk: <b>{feature.risk}</b></div>
            <details>
              <summary>Permissions, data and limitations</summary>
              <dl className="feature-centre-details">
                <div><dt>Permission / consent</dt><dd>{feature.permissions}</dd></div>
                <div><dt>Data handling</dt><dd>{feature.data}</dd></div>
                <div><dt>Limits and dependencies</dt><dd>{feature.limits}</dd></div>
              </dl>
            </details>
          </article>
        ))}
      </div>
      <p className="feature-centre-footnote">
        Statuses describe the current code and documented implementation boundary, not real-device certification. You remain responsible for actual user testing and release acceptance.
      </p>
    </section>
  );
}
