import { ExternalLink, Globe2, Mail, MessageSquareText, PenLine, Search } from "lucide-react";

type IntegrationLink = {
  name: string;
  description: string;
  url: string;
  category: "AI assistants" | "Productivity" | "Research";
  icon: typeof Globe2;
};

const integrations: IntegrationLink[] = [
  { name: "ChatGPT", description: "Open the ChatGPT website. Existing subscriptions and API access are governed separately by OpenAI.", url: "https://chatgpt.com/", category: "AI assistants", icon: MessageSquareText },
  { name: "Claude", description: "Open Claude in your browser; account and plan requirements are set by Anthropic.", url: "https://claude.ai/", category: "AI assistants", icon: MessageSquareText },
  { name: "Gemini", description: "Open Google's Gemini assistant; availability depends on your account and region.", url: "https://gemini.google.com/", category: "AI assistants", icon: MessageSquareText },
  { name: "Perplexity", description: "Open Perplexity for web-grounded research and answers.", url: "https://www.perplexity.ai/", category: "Research", icon: Search },
  { name: "Google Docs", description: "Open the online document editor. Sign-in and internet access may be required.", url: "https://docs.google.com/", category: "Productivity", icon: PenLine },
  { name: "Outlook", description: "Open Outlook on the web. Connectors and account permissions are separate optional features.", url: "https://outlook.office.com/mail/", category: "Productivity", icon: Mail },
];

const categories: IntegrationLink["category"][] = ["AI assistants", "Productivity", "Research"];

export default function IntegrationDirectory() {
  return <section className="model-box integration-directory" id="integration-directory" tabIndex={-1} aria-labelledby="integration-directory-title">
    <div className="section-heading">
      <div><h2 id="integration-directory-title"><Globe2 size={18}/> Open application directory</h2><p>Use the applications you already prefer. UJANI does not require you to replace them.</p></div>
      <span className="muted">OPTIONAL LINKS</span>
    </div>
    <p className="integration-notice">These links open the provider's website. They do not connect accounts, transfer UJANI content, or use a paid subscription through an API. Those capabilities require a separately supported and authorised integration.</p>
    {categories.map((category) => <div className="integration-category" key={category}>
      <h3>{category}</h3>
      <div className="integration-grid">
        {integrations.filter((item) => item.category === category).map((item) => {
          const Icon = item.icon;
          return <a className="integration-link" key={item.name} href={item.url} target="_blank" rel="noopener noreferrer">
            <span className="integration-icon"><Icon size={17}/></span>
            <span className="integration-copy"><b>{item.name}</b><small>{item.description}</small><em>Open website <ExternalLink size={11}/></em></span>
          </a>;
        })}
      </div>
    </div>)}
    <small className="muted">Providers operate independently and may change availability, pricing, account requirements and privacy terms. Review their terms before sharing sensitive information.</small>
  </section>;
}
