import catalogue from "./model-catalog.json";

type ModelCatalogueProps = {
  totalMemoryMb?: number;
  availableMemoryMb?: number;
  os?: string;
  onChooseModel?: (model: string) => void;
};

function recommendation(total?: number, available?: number, os?: string) {
  if (!Number.isFinite(total) || !Number.isFinite(available) || !os) {
    return { model: null, title: "No model download recommended yet", reason: "UJANI cannot verify this device's operating system and available memory. You can still use UJANI's built-in features and choose a model later." };
  }
  const platform = os.toLowerCase();
  if (platform.includes("android") || platform.includes("ios") || platform.includes("iphone") || platform.includes("ipad")) {
    return { model: null, title: "No desktop model runtime recommendation", reason: "Ollama desktop support is not assumed on mobile. UJANI will offer a mobile model only after that exact runtime and device class have passed validation." };
  }
  const totalRam = total as number;
  const freeRam = available as number;
  if (totalRam < 6144) {
    return { model: null, title: "Recommended default: no local model", reason: "On a 4 GB computer, keep memory available for the operating system and UJANI. Native OS-powered features remain available without downloading a model." };
  }
  if (totalRam >= 24576 && freeRam >= 10240) {
    return { model: "qwen3:8b", title: "Recommended candidate: Qwen3 8B", reason: "This device reports at least 24 GiB total RAM and 10 GiB available RAM. Start with a 4K context and one inference at a time." };
  }
  if (totalRam >= 16384 && freeRam >= 6144) {
    return { model: "qwen3:4b", title: "Recommended candidate: Qwen3 4B", reason: "This device reports at least 16 GiB total RAM and 6 GiB available RAM. Start with a 4K context and one inference at a time." };
  }
  if (totalRam >= 8192 && freeRam >= 3584) {
    return { model: "qwen3:1.7b", title: "Recommended candidate: Qwen3 1.7B", reason: "This device reports at least 8 GiB total RAM and 3.5 GiB available RAM. Start with a 4K context and one inference at a time." };
  }
  if (totalRam >= 6144 && freeRam >= 2560) {
    return { model: "qwen3:0.6b", title: "Recommended candidate: Qwen3 0.6B", reason: "This device reports at least 6 GiB total RAM and 2.5 GiB available RAM. This is the lightest general-purpose chat candidate; start with a 4K context." };
  }
  return { model: null, title: "Recommended default: no local model", reason: "Available memory is below UJANI's conservative starting threshold. Keep model installation optional and use native OS-powered features." };
}

export default function ModelCatalogue({ totalMemoryMb, availableMemoryMb, os, onChooseModel }: ModelCatalogueProps) {
  const choice = recommendation(totalMemoryMb, availableMemoryMb, os);
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
        <small className="muted">This is a conservative starting recommendation, not a performance guarantee. UJANI-managed downloads are currently disabled until exact artifact and Ollama storage preflight can be verified. Selecting a candidate only fills its name for manual setup; it does not download or select it for inference.</small>
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
            <p><b>Useful for:</b> {model.tasks.join(", ").replace(/_/g, " ")}.</p>
            <p><b>Limitations:</b> {model.limitations.join(" ")}</p>
            <p><b>Storage:</b> Exact download and installed sizes depend on the verified quantised artifact and will be shown before download.</p>
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
