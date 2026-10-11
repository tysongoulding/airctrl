import React, { Component, ErrorInfo, ReactNode } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught error:", error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      return (
        <div style={{ padding: "32px", color: "#f87171", backgroundColor: "#090d16", height: "100vh", fontFamily: "sans-serif" }}>
          <h2 style={{ fontSize: "20px", fontWeight: "bold", marginBottom: "12px" }}>airctrl UI Render Error</h2>
          <pre style={{ background: "#1e293b", padding: "16px", borderRadius: "8px", overflow: "auto", color: "#cbd5e1" }}>
            {this.state.error?.stack || this.state.error?.message || "Unknown error"}
          </pre>
          <button
            onClick={() => window.location.reload()}
            style={{ marginTop: "16px", padding: "8px 16px", background: "#4f46e5", color: "white", borderRadius: "6px", border: "none", cursor: "pointer" }}
          >
            Reload Window
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

const rootElement = document.getElementById("root");
if (rootElement) {
  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      <ErrorBoundary>
        <App />
      </ErrorBoundary>
    </React.StrictMode>
  );
}
