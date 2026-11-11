type Props = { onChoose: (prompt: string) => void };

const actions = [
  { label: "Improve grammar", prompt: "Correct grammar and spelling. Preserve my meaning and facts; do not add unsupported information.\n\n[paste your text here]" },
  { label: "Professional rewrite", prompt: "Rewrite in a clear professional tone. Preserve facts, names, dates and commitments; do not invent details.\n\n[paste your text here]" },
  { label: "Draft an email", prompt: "Draft a concise email using only facts I provide. Leave placeholders for missing details rather than inventing them.\n\nPurpose and key points: [describe here]" },
  { label: "Summarise", prompt: "Summarise the text into key points. Separate facts from interpretation and do not introduce new claims.\n\n[paste your text here]" },
  { label: "Explain simply", prompt: "Explain the material in plain language while preserving qualifications. Identify ambiguity rather than guessing.\n\n[paste your text here]" },
  { label: "Translate", prompt: "Translate into [target language]. Preserve names, numbers, dates and formatting. Do not summarise or add information.\n\n[paste your text here]" },
];

export default function EverydayWritingActions({ onChoose }: Props) {
  return <section aria-labelledby="everyday-writing-title" style={{ margin: "1rem 0", padding: "1rem", border: "1px solid var(--border, #d1d5db)", borderRadius: "12px" }}>
    <h2 id="everyday-writing-title" style={{ margin: 0, fontSize: "1.1rem" }}>Everyday writing shortcuts</h2>
    <p style={{ margin: "0.35rem 0 0.75rem", opacity: 0.8 }}>Choose a template to place it in the task composer. Review and edit it before running a task.</p>
    <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(145px, 1fr))", gap: "0.5rem" }}>
      {actions.map((action) => <button key={action.label} type="button" onClick={() => onChoose(action.prompt)} style={{ minHeight: "42px", padding: "0.55rem 0.7rem", border: "1px solid var(--border, #d1d5db)", borderRadius: "8px", background: "transparent", color: "inherit", cursor: "pointer", textAlign: "left" }}>{action.label}</button>)}
    </div>
    <p style={{ margin: "0.75rem 0 0", fontSize: "0.8rem", opacity: 0.75 }}>Templates do not submit tasks, enable cloud access or send content externally.</p>
  </section>;
}
