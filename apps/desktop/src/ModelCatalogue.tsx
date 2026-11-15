import catalogue from "./model-catalog.json";
import { recommendDefaultModel } from "./modelRecommendation.js";

type ModelCatalogueProps = {
  totalMemoryMb?: number;
  availableMemoryMb?: number;
  sampledAtUnixMs?: number;
  os?: string;
  onChooseModel?: (model: string) => void;
};

export default function ModelCatalogue({ totalMemoryMb, availableMemoryMb, sampledAtUnixMs, os, onChooseModel }: ModelCatalogueProps) {
  const choice = recommendDefaultModel(totalMemoryMb, availableMemoryMb, os, sampledAtUnixMs, Date.now());
  return (
    <section className="install-guide" aria-labelledby="local-model-catalogue-title">
      <b id="local-model-catalogue-title">Understand local AI models before choosing</b>
      <div className="model-recommendation" aria-live="polite">
        <b>{choice.title}</b>
        <p>{choice.reason}</p>
        <p>Desktop runtime recommendation: Ollama, only on supported desktop systems. Ollama is separate from the AI model and is not bundled or installed automatically.</p>
        {choice.model && onChooseModel && (
          <button className="model-run" type="button" onClick={() => onChooseModel(choice.model!)}>
            Use {choice.model} as my manual Ollama setup choice
          </button>
        )}
        <small className="muted">This is a conservative starting recommendation, not a performance guarantee. Available-memory evidence older than five minutes is ignored. Selecting a candidate only fills its model identifier into local setup; it does not install, download or select it for inference. UJANI-managed in-app model pulls are currently disabled because the native preflight cannot yet verify the exact upstream artifact and the Ollama daemon's actual model-store path and free space. If you choose to install a model, use your trusted Ollama interface or CLI directly, verify the exact registry artifact and licence, check available disk space, then refresh UJANI's model inventory. UJANI does not independently pin or verify the quantised artifact, publisher digest or final installed size.</small>
      </div>
      <p>These are optional model candidates. Exact artifact, quantisation, publisher, licence, digest, download size and disk-space requirements must be verified before any manual installation.</p>
      <div className="model-catalogue-list">
        {catalogue.models.map((model) => (
          <details key={model.id} className="model-catalogue-card">
            <summary>
              <strong>{model.display_name}</strong>
              <span> · {model.planning_ram_gib.minimum} GiB minimum planning guide / {model.planning_ram_gib.recommended} GiB recommended guide</span>
            </summary>
            <p>{model.purpose}</p>
            <p><b>Publisher / family:</b> {model.publisher} · {model.family}</p>
            <p><b>Runtime artifact:</b> {model.runtime_artifact}</p>
            <p><b>Useful for:</b> {model.tasks.join(", ").replace(/_/g, " ")}.</p>
            <p><b>Candidate platforms:</b> Windows, macOS and Linux with a compatible, installed Ollama desktop runtime. Android and iOS are not supported by this desktop profile; UJANI must verify the exact runtime and architecture before recommending installation.</p>
            <p><b>Limitations:</b> {model.limitations.join(" ")}</p>
            <p><b>Storage:</b> Exact download and installed sizes vary by registry artifact and quantisation. Check the current Ollama registry entry and local free disk space before using the separate download action; this catalogue does not yet provide a verified size estimate.</p>
            <p><b>Licence:</b> {model.license}</p>
            <small className="muted">Candidate only · {model.runtime_status.replace(/_/g, " ")} · RAM figures are advisory planning guidance, not a fit guarantee.</small>
          </details>
        ))}
      </div>
      <b>Specialist local tools</b>
      <div className="model-catalogue-list">
        {catalogue.specialist_tools.map((tool) => (
          <details key={tool.id} className="model-catalogue-card">
            <summary><strong>{tool.display_name}</strong> · {tool.category.replace(/_/g, " ")}</summary>
            <p>{tool.purpose}</p>
            <p><b>Limitations:</b> {tool.limitations.join(" ")}</p>
            <small className="muted">Separate runtime integration required · Not a general chat model.</small>
          </details>
        ))}
      </div>
    </section>
  );
}
