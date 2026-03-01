import { createElement, type ReactNode } from "react";

interface MarkdownContentProps {
  content: string;
  className?: string;
}

function parseInline(text: string): ReactNode[] {
  const parts: ReactNode[] = [];
  const tokenRe = /(\*\*[^*]+\*\*|\*[^*]+\*|~~[^~]+~~|`[^`]+`|\[[^\]]+\]\([^)]+\))/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  let key = 0;

  while ((match = tokenRe.exec(text))) {
    if (match.index > lastIndex) {
      parts.push(text.slice(lastIndex, match.index));
    }

    const token = match[0];
    if (token.startsWith("**") && token.endsWith("**")) {
      parts.push(<strong key={`md-i-${key++}`}>{token.slice(2, -2)}</strong>);
    } else if (token.startsWith("*") && token.endsWith("*")) {
      parts.push(<em key={`md-i-${key++}`}>{token.slice(1, -1)}</em>);
    } else if (token.startsWith("~~") && token.endsWith("~~")) {
      parts.push(<del key={`md-i-${key++}`}>{token.slice(2, -2)}</del>);
    } else if (token.startsWith("`") && token.endsWith("`")) {
      parts.push(<code key={`md-i-${key++}`}>{token.slice(1, -1)}</code>);
    } else if (token.startsWith("[")) {
      const m = token.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
      if (m) {
        parts.push(
          <a key={`md-i-${key++}`} href={m[2]} target="_blank" rel="noreferrer">
            {m[1]}
          </a>
        );
      } else {
        parts.push(token);
      }
    } else {
      parts.push(token);
    }

    lastIndex = tokenRe.lastIndex;
  }

  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }

  return parts;
}

function renderBlock(lines: string[]): ReactNode[] {
  const nodes: ReactNode[] = [];
  let i = 0;
  let key = 0;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    if (!trimmed) {
      i += 1;
      continue;
    }

    const heading = trimmed.match(/^(#{1,6})\s+(.+)$/);
    if (heading) {
      const level = heading[1].length;
      const text = heading[2];
      const children = parseInline(text);
      nodes.push(createElement(`h${level}`, { key: `md-b-${key++}` }, children));
      i += 1;
      continue;
    }

    if (/^(-{3,}|\*{3,})$/.test(trimmed)) {
      nodes.push(<hr key={`md-b-${key++}`} />);
      i += 1;
      continue;
    }

    if (/^>\s?/.test(trimmed)) {
      const quoteLines: string[] = [];
      while (i < lines.length && /^>\s?/.test(lines[i].trim())) {
        quoteLines.push(lines[i].trim().replace(/^>\s?/, ""));
        i += 1;
      }
      nodes.push(<blockquote key={`md-b-${key++}`}>{parseInline(quoteLines.join(" "))}</blockquote>);
      continue;
    }

    const isTableRow = (v: string) => v.includes("|");
    const isTableSep = (v: string) => {
      const parts = v
        .split("|")
        .map((p) => p.trim())
        .filter(Boolean);
      return parts.length > 0 && parts.every((p) => /^:?-{3,}:?$/.test(p));
    };
    const splitCells = (v: string) =>
      v
        .split("|")
        .map((c) => c.trim())
        .filter((_, idx, arr) => !(idx === 0 && arr[idx] === "") && !(idx === arr.length - 1 && arr[idx] === ""));

    if (i + 1 < lines.length && isTableRow(trimmed) && isTableSep(lines[i + 1].trim())) {
      const headerCells = splitCells(trimmed);
      i += 2;
      const rowLines: string[] = [];
      while (i < lines.length && lines[i].trim() && isTableRow(lines[i].trim())) {
        rowLines.push(lines[i].trim());
        i += 1;
      }
      nodes.push(
        <table key={`md-b-${key++}`}>
          <thead>
            <tr>
              {headerCells.map((cell, idx) => (
                <th key={`md-th-${idx}`}>{parseInline(cell)}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rowLines.map((row, rowIdx) => {
              const cells = splitCells(row);
              return (
                <tr key={`md-tr-${rowIdx}`}>
                  {headerCells.map((_, colIdx) => (
                    <td key={`md-td-${rowIdx}-${colIdx}`}>{parseInline(cells[colIdx] ?? "")}</td>
                  ))}
                </tr>
              );
            })}
          </tbody>
        </table>
      );
      continue;
    }

    if (/^[-*+]\s+/.test(trimmed)) {
      const items: ReactNode[] = [];
      while (i < lines.length && /^[-*+]\s+/.test(lines[i].trim())) {
        items.push(<li key={`md-li-${key++}`}>{parseInline(lines[i].trim().replace(/^[-*+]\s+/, ""))}</li>);
        i += 1;
      }
      nodes.push(<ul key={`md-b-${key++}`}>{items}</ul>);
      continue;
    }

    if (/^\d+\.\s+/.test(trimmed)) {
      const items: ReactNode[] = [];
      while (i < lines.length && /^\d+\.\s+/.test(lines[i].trim())) {
        items.push(<li key={`md-ol-${key++}`}>{parseInline(lines[i].trim().replace(/^\d+\.\s+/, ""))}</li>);
        i += 1;
      }
      nodes.push(<ol key={`md-b-${key++}`}>{items}</ol>);
      continue;
    }

    const para: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() &&
      !/^(#{1,6})\s+/.test(lines[i].trim()) &&
      !/^[-*+]\s+/.test(lines[i].trim()) &&
      !/^\d+\.\s+/.test(lines[i].trim()) &&
      !/^>\s+/.test(lines[i].trim()) &&
      !/^(-{3,}|\*{3,})$/.test(lines[i].trim())
    ) {
      para.push(lines[i].trim());
      i += 1;
    }
    nodes.push(<p key={`md-b-${key++}`}>{parseInline(para.join(" "))}</p>);
  }

  return nodes;
}

export function MarkdownContent({ content, className }: MarkdownContentProps) {
  const chunks = content.split(/```/);
  const nodes: ReactNode[] = [];
  let key = 0;

  chunks.forEach((chunk, idx) => {
    if (idx % 2 === 1) {
      const codeLines = chunk.split("\n");
      const first = codeLines[0]?.trim() ?? "";
      const hasLang = /^[a-zA-Z0-9_-]+$/.test(first);
      const code = hasLang ? codeLines.slice(1).join("\n") : chunk;
      const lang = hasLang ? first : "";
      nodes.push(
        <pre key={`md-code-${key++}`}>
          <code data-lang={lang}>{code.replace(/\n$/, "")}</code>
        </pre>
      );
    } else {
      const lines = chunk.split("\n");
      nodes.push(...renderBlock(lines).map((n) => <div key={`md-text-${key++}`}>{n}</div>));
    }
  });

  return <div className={`markdown-content ${className ?? ""}`.trim()}>{nodes}</div>;
}
