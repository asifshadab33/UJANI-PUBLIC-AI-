import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

type BoundaryState = { error: Error | null };
class AppErrorBoundary extends React.Component<React.PropsWithChildren, BoundaryState> {
  state: BoundaryState = { error: null };
  static getDerivedStateFromError(error: Error): BoundaryState { return { error }; }
  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("UJANI workspace render failure", error, info.componentStack);
  }
  render() {
    if (this.state.error) {
      return <main style={{ minHeight: "100vh", padding: 32, background: "#101812", color: "#edf4ee", fontFamily: "sans-serif" }}>
        <h1>UJANI could not render this workspace</h1>
        <p>The application caught a UI error instead of leaving a blank or frozen screen.</p>
        <pre style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere", color: "#ffcfaa" }}>{this.state.error.message}</pre>
        <button onClick={() => window.location.reload()}>Reload UJANI</button>
      </main>;
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode><AppErrorBoundary><App /></AppErrorBoundary></React.StrictMode>
);