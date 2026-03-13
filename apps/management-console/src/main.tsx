import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";
import { AvatarCharacterProvider } from "./hooks/AvatarContext";

import ErrorBoundary from "./components/ErrorBoundary";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <AvatarCharacterProvider>
        <App />
      </AvatarCharacterProvider>
    </ErrorBoundary>
  </React.StrictMode>,
);
