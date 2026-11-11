type Integration = { name: string; category: string; description: string; url: string };

const integrations: Integration[] = [
  { name: "LibreOffice", category: "Documents", description: "Open-source office suite for documents, spreadsheets and presentations.", url: "https://www.libreoffice.org/" },
  { name: "Thunderbird", category: "Email", description: "Open-source email and calendar client.", url: "https://www.thunderbird.net/" },
  { name: "Nextcloud", category: "Files and collaboration", description: "Self-hostable file sync, sharing and collaboration platform.", url: "https://nextcloud.com/" },
  { name: "Jitsi Meet", category: "Meetings", description: "Browser-based video meetings; review hosting privacy and deployment settings.", url: "https://meet.jit.si/" },
  { name: "Element", category: "Team communication", description: "Open-source client for the Matrix communication network.", url: "https://element.io/" },
  { name: "CryptPad", category: "Collaborative writing", description: "Privacy-focused collaborative documents and spreadsheets.", url: "https://cryptpad.org/" },
];

export default function IntegrationDirectory() {
  return <section id="integration-directory" tabIndex={-1} aria-labelledby="integration-directory-title" style={{ margin: "1rem 0 2rem", padding: "1rem", border: "1px solid var(--border, #d1d5db)", borderRadius: "12px", scrollMarginTop: "1rem" }}>
    <h2 id="integration-directory-title" style={{ margin: 0, fontSize: "1.1rem" }}>External integration directory</h2>
    <p style={{ margin: "0.35rem 0 0.75rem", opacity: 0.8 }}>These are optional links, not connected accounts. UJANI does not sign in, request credentials or share data through this directory.</p>
    <ul style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "0.75rem", listStyle: "none", padding: 0, margin: 0 }}>
      {integrations.map((item) => <li key={item.name} style={{ padding: "0.75rem", border: "1px solid var(--border, #d1d5db)", borderRadius: "8px" }}>
        <div style={{ fontWeight: 600 }}>{item.name}</div>
        <div style={{ fontSize: "0.78rem", opacity: 0.75 }}>{item.category}</div>
        <p style={{ margin: "0.4rem 0", fontSize: "0.88rem" }}>{item.description}</p>
        <a href={item.url} target="_blank" rel="noopener noreferrer">Visit {item.name}</a>
      </li>)}
    </ul>
    <p style={{ margin: "0.75rem 0 0", fontSize: "0.8rem", opacity: 0.75 }}>External providers have their own accounts, availability, terms and privacy policies. Opening a link leaves UJANI; check the destination before sharing information.</p>
  </section>;
}
