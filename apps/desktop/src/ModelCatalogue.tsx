import catalogue from "./model-catalog.json";

export default function ModelCatalogue() {
  return (
    <section className="install-guide" aria-labelledby="local-model-catalogue-title">
      <b id="local-model-catalogue-title">Understand local AI models before choosing</b>
      <p>These are optional model candidates. UJANI does not download anything from this catalogue. Exact artifact, quantisation, publisher, licence, digest, download size and disk-space requirements must be verified before a separate download confirmation can be offered.</p>
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
