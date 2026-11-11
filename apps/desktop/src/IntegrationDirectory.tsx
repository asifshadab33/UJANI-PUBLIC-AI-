type Integration = { name: string; description: string; url: string; access: "Free tier" | "Open source" | "Public resource" };

type Category = { name: string; items: Integration[] };

const categories: Category[] = [
  { name: "Outreach and email", items: [
    { name: "Mailchimp", access: "Free tier", description: "Email campaigns and audience tools; check current sending and contact limits.", url: "https://mailchimp.com/" },
    { name: "Brevo", access: "Free tier", description: "Email and customer messaging tools; free-plan limits and features may change.", url: "https://www.brevo.com/" },
    { name: "MailerLite", access: "Free tier", description: "Email marketing and newsletter tools; verify current subscriber and sending limits.", url: "https://www.mailerlite.com/" },
    { name: "Listmonk", access: "Open source", description: "Self-hosted mailing-list and newsletter manager; hosting and delivery are your responsibility.", url: "https://listmonk.app/" },
  ]},
  { name: "Market intelligence", items: [
    { name: "Google Trends", access: "Public resource", description: "Explore search-interest patterns by topic, region and time.", url: "https://trends.google.com/" },
    { name: "Similarweb", access: "Free tier", description: "Website traffic and market research previews; detailed data may require a paid plan.", url: "https://www.similarweb.com/" },
    { name: "BuiltWith", access: "Free tier", description: "Look up technologies used by public websites; free lookup limits apply.", url: "https://builtwith.com/" },
    { name: "OpenCorporates", access: "Public resource", description: "Search company-register information from participating jurisdictions; coverage varies.", url: "https://opencorporates.com/" },
  ]},
  { name: "News and current affairs", items: [
    { name: "Google News", access: "Public resource", description: "Browse and compare reporting from multiple publishers.", url: "https://news.google.com/" },
    { name: "BBC News", access: "Public resource", description: "International and regional news reporting; availability varies by location.", url: "https://www.bbc.com/news" },
    { name: "Associated Press", access: "Public resource", description: "News reporting and explainers from the AP newsroom.", url: "https://apnews.com/" },
    { name: "Reuters", access: "Public resource", description: "Global news and business reporting; some content may be restricted.", url: "https://www.reuters.com/" },
  ]},
  { name: "Documents and PDF", items: [
    { name: "LibreOffice", access: "Open source", description: "Desktop office suite for documents, spreadsheets and presentations.", url: "https://www.libreoffice.org/" },
    { name: "PDF24 Tools", access: "Free tier", description: "Browser-based PDF conversion and editing utilities; review upload/privacy settings.", url: "https://tools.pdf24.org/" },
    { name: "Stirling PDF", access: "Open source", description: "Self-hostable PDF tools; self-hosting keeps files under your infrastructure.", url: "https://www.stirlingpdf.com/" },
    { name: "PDFsam Basic", access: "Open source", description: "Desktop PDF split, merge, rotate and extract tools.", url: "https://pdfsam.org/pdfsam-basic/" },
  ]},
  { name: "Data and research", items: [
    { name: "Our World in Data", access: "Public resource", description: "Research and open datasets on global development and public-interest topics.", url: "https://ourworldindata.org/" },
    { name: "World Bank Open Data", access: "Public resource", description: "Development indicators and country-level datasets.", url: "https://data.worldbank.org/" },
    { name: "UNData", access: "Public resource", description: "Statistical datasets from United Nations sources.", url: "https://data.un.org/" },
    { name: "OpenAlex", access: "Public resource", description: "Open scholarly metadata for works, authors, institutions and topics.", url: "https://openalex.org/" },
  ]},
  { name: "Meetings and collaboration", items: [
    { name: "Jitsi Meet", access: "Open source", description: "Open-source video meeting software with a public hosted instance.", url: "https://meet.jit.si/" },
    { name: "Google Meet", access: "Free tier", description: "Browser-based video meetings; account and feature limits may apply.", url: "https://meet.google.com/" },
    { name: "Zoom", access: "Free tier", description: "Video meetings with time and feature limits on the basic plan.", url: "https://zoom.us/" },
    { name: "BigBlueButton", access: "Open source", description: "Virtual classroom and conferencing software; requires a hosted service.", url: "https://bigbluebutton.org/" },
  ]},
  { name: "File transfer", items: [
    { name: "SwissTransfer", access: "Free tier", description: "Send large files using a hosted transfer service; retention and size limits apply.", url: "https://www.swisstransfer.com/" },
    { name: "Wormhole", access: "Free tier", description: "Share files with encrypted transfer links; verify current size and expiry limits.", url: "https://wormhole.app/" },
    { name: "LocalSend", access: "Open source", description: "Transfer files directly between nearby devices on a local network.", url: "https://localsend.org/" },
    { name: "PairDrop", access: "Open source", description: "Browser-based peer-to-peer file sharing; both devices need compatible connectivity.", url: "https://pairdrop.net/" },
  ]},
  { name: "Writing and editing", items: [
    { name: "LanguageTool", access: "Free tier", description: "Grammar and style checking; free-tier language and length limits apply.", url: "https://languagetool.org/" },
    { name: "Hemingway Editor", access: "Free tier", description: "Browser-based readability and writing-style feedback.", url: "https://hemingwayapp.com/" },
    { name: "CryptPad", access: "Open source", description: "Privacy-focused collaborative documents and spreadsheets.", url: "https://cryptpad.org/" },
    { name: "Etherpad", access: "Open source", description: "Real-time collaborative text editor; use a trusted hosted instance or self-host.", url: "https://etherpad.org/" },
  ]},
];

export default function IntegrationDirectory() {
  return <section id="integration-directory" tabIndex={-1} aria-labelledby="integration-directory-title" style={{ margin: "1rem 0 2rem", padding: "1rem", border: "1px solid var(--border, #d1d5db)", borderRadius: "12px", scrollMarginTop: "1rem" }}>
    <h2 id="integration-directory-title" style={{ margin: 0, fontSize: "1.1rem" }}>External tools directory</h2>
    <p style={{ margin: "0.35rem 0 0.75rem", opacity: 0.8 }}>Optional links only — these are not connected accounts. No credentials or user data are sent by UJANI when this directory is displayed. Entries are free tiers, open-source projects or public resources; providers may change limits, terms or availability.</p>
    <div style={{ display: "grid", gap: "1rem" }}>
      {categories.map((category) => <section key={category.name} aria-labelledby={`integration-${category.name.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`}>
        <h3 id={`integration-${category.name.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`} style={{ margin: "0 0 0.5rem", fontSize: "1rem" }}>{category.name}</h3>
        <ul style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "0.65rem", listStyle: "none", padding: 0, margin: 0 }}>
          {category.items.map((item) => <li key={item.name} style={{ padding: "0.7rem", border: "1px solid var(--border, #d1d5db)", borderRadius: "8px" }}>
            <div style={{ fontWeight: 600 }}>{item.name}</div>
            <div style={{ fontSize: "0.75rem", opacity: 0.75 }}>{item.access}</div>
            <p style={{ margin: "0.35rem 0", fontSize: "0.85rem" }}>{item.description}</p>
            <a href={item.url} target="_blank" rel="noopener noreferrer">Visit {item.name}</a>
          </li>)}
        </ul>
      </section>)}
    </div>
    <p style={{ margin: "0.75rem 0 0", fontSize: "0.8rem", opacity: 0.75 }}>External providers have their own accounts, terms, privacy policies and retention practices. Opening a link leaves UJANI. Check the destination and avoid sharing sensitive information unless you trust the provider.</p>
  </section>;
}
