import React, { Fragment } from "react";

type InlinePart = { kind: "text" | "code" | "strong" | "em" | "link"; value: string; href?: string };

function safeHttpUrl(value: string): string | null {
  try {
    const url = new URL(value);
    return (url.protocol === "https:" || url.protocol === "http:") && !url.username && !url.password ? url.href : null;
  } catch {
    return null;
  }
}

function parseInline(source: string): InlinePart[] {
  const pattern = /\x60[^\x60]+\x60|\*\*[^*]+\*\*|\*[^*]+\*|\[[^\]]+\]\(https?:\/\/[^\s)]+\)/g;
  const parts: InlinePart[] = [];
  let cursor = 0;
  for (const match of source.matchAll(pattern)) {
    const index = match.index ?? 0;
    if (index > cursor) parts.push({ kind: "text", value: source.slice(cursor, index) });
    const token = match[0];
    if (token.startsWith("\x60")) parts.push({ kind: "code", value: token.slice(1, -1) });
    else if (token.startsWith("**")) parts.push({ kind: "strong", value: token.slice(2, -2) });
    else if (token.startsWith("*")) parts.push({ kind: "em", value: token.slice(1, -1) });
    else {
      const link = /^\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)$/.exec(token);
      const href = link ? safeHttpUrl(link[2]) : null;
      parts.push(href && link
        ? { kind: "link", value: link[1], href }
        : { kind: "text", value: token });
    }
    cursor = index + token.length;
  }
  if (cursor < source.length) parts.push({ kind: "text", value: source.slice(cursor) });
  return parts;
}

function InlineText({ value }: { value: string }) {
  return <>{parseInline(value).map((part, index) => {
    switch (part.kind) {
      case "code": return <code key={index}>{part.value}</code>;
      case "strong": return <strong key={index}>{part.value}</strong>;
      case "em": return <em key={index}>{part.value}</em>;
      case "link": return <a key={index} href={part.href} target="_blank" rel="noopener noreferrer">{part.value}</a>;
      default: return <Fragment key={index}>{part.value}</Fragment>;
    }
  })}</>;
}

/**
 * Limited Markdown renderer for assistant output.
 * It creates React text/elements only: no HTML parsing or dangerouslySetInnerHTML.
 * Only credential-free HTTP(S) links are clickable; raw HTML and unsafe URL schemes remain text.
 */
function boundedUtf8Text(content: string, maximumBytes: number): string {
  let bytes = 0;
  let end = 0;
  for (const character of content) {
    const first = character.charCodeAt(0);
    const width = character.length === 2 ? 4 : first <= 0x7f ? 1 : first <= 0x7ff ? 2 : 3;
    if (bytes + width > maximumBytes) break;
    bytes += width;
    end += character.length;
  }
  return content.slice(0, end);
}

function ChatMessageContentImpl({ content }: { content: string }) {
  const source = boundedUtf8Text(content, 128 * 1024);
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const blocks: React.ReactNode[] = [];
  let index = 0;
  let key = 0;

  while (index < lines.length) {
    const line = lines[index];
    if (!line.trim()) { index += 1; continue; }

    const openingFence = /^ {0,3}(`{3,})(.*)$/.exec(line);
    if (openingFence && !openingFence[2].includes("`")) {
      const fenceLength = openingFence[1].length;
      const language = openingFence[2].trim().replace(/[^a-zA-Z0-9_-]/g, "").slice(0, 24);
      index += 1;
      const code: string[] = [];
      while (index < lines.length) {
        const closingFence = /^ {0,3}(`+)\s*$/.exec(lines[index]);
        if (closingFence && closingFence[1].length >= fenceLength) break;
        code.push(lines[index]);
        index += 1;
      }
      if (index < lines.length) index += 1;
      blocks.push(<pre key={key++}><code data-language={language || undefined}>{code.join("\n")}</code></pre>);
      continue;
    }

    const heading = /^(#{1,3})\s+(.+)$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      const text = <InlineText value={heading[2]} />;
      blocks.push(level === 1 ? <h3 key={key++}>{text}</h3> : level === 2 ? <h4 key={key++}>{text}</h4> : <h5 key={key++}>{text}</h5>);
      index += 1;
      continue;
    }

    if (/^\s*[-*]\s+/.test(line)) {
      const items: string[] = [];
      while (index < lines.length && /^\s*[-*]\s+/.test(lines[index])) {
        items.push(lines[index].replace(/^\s*[-*]\s+/, ""));
        index += 1;
      }
      blocks.push(<ul key={key++}>{items.map((item, itemIndex) => <li key={itemIndex}><InlineText value={item} /></li>)}</ul>);
      continue;
    }

    if (/^\s*\d+[.)]\s+/.test(line)) {
      const items: string[] = [];
      while (index < lines.length && /^\s*\d+[.)]\s+/.test(lines[index])) {
        items.push(lines[index].replace(/^\s*\d+[.)]\s+/, ""));
        index += 1;
      }
      blocks.push(<ol key={key++}>{items.map((item, itemIndex) => <li key={itemIndex}><InlineText value={item} /></li>)}</ol>);
      continue;
    }

    if (/^>\s?/.test(line)) {
      const quote: string[] = [];
      while (index < lines.length && /^>\s?/.test(lines[index])) {
        quote.push(lines[index].replace(/^>\s?/, ""));
        index += 1;
      }
      blocks.push(<blockquote key={key++}>{quote.map((item, itemIndex) => <p key={itemIndex}><InlineText value={item} /></p>)}</blockquote>);
      continue;
    }

    // No block rule matched this line. Consume it so the outer loop always progresses.
    const paragraph: string[] = [line];
    index += 1;
    while (index < lines.length && lines[index].trim()
      && !lines[index].trimStart().startsWith("\x60\x60\x60")
      && !/^(#{1,3})\s+/.test(lines[index])
      && !/^\s*[-*]\s+/.test(lines[index])
      && !/^\s*\d+[.)]\s+/.test(lines[index])
      && !/^>\s?/.test(lines[index])) {
      paragraph.push(lines[index]);
      index += 1;
    }
    blocks.push(<p key={key++}><InlineText value={paragraph.join("\n")} /></p>);
  }

  return <div className="chat-markdown">{blocks}</div>;
}

export default React.memo(ChatMessageContentImpl);
