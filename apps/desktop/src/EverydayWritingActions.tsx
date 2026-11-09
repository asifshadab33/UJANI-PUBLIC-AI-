type Props = { onSelect: (prompt: string) => void };

const actions = [
  { label: "Correct grammar", prompt: "Correct the grammar, spelling and punctuation in the text below. Preserve its meaning and original tone. Return the corrected text only.\n\nPaste your text here:" },
  { label: "Write an email", prompt: "Draft a clear, professional email using only the facts I provide. Do not invent names, dates, promises or commitments. Include a suitable subject line.\n\nPurpose and details:" },
  { label: "Make professional", prompt: "Rewrite the text below in a polished, professional tone. Preserve all facts and meaning. Do not add unsupported information.\n\nPaste your text here:" },
  { label: "Summarise", prompt: "Summarise the text below into concise key points. Preserve important qualifications and do not add facts.\n\nPaste your text here:" },
  { label: "Explain simply", prompt: "Explain the text below in plain, easy-to-understand language. Preserve the original meaning and identify any uncertainty.\n\nPaste your text here:" },
  { label: "Translate", prompt: "Translate the text below into [target language]. Preserve meaning, names, numbers and tone.\n\nPaste your text here:" },
];

export default function EverydayWritingActions({ onSelect }: Props) {
  return <div className="everyday-writing-actions" aria-label="Everyday writing shortcuts">
    <div><b>Everyday writing shortcuts</b><small>Choose a task, add your text, then run it with your selected local model.</small></div>
    <div className="writing-action-list">
      {actions.map((action) => <button type="button" className="writing-action" key={action.label} onClick={() => onSelect(action.prompt)}>{action.label}</button>)}
    </div>
  </div>;
}
