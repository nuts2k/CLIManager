import "./App.css";

const backendCommands = [
  "list_providers",
  "get_provider",
  "create_provider",
  "update_provider",
  "delete_provider",
  "get_local_settings",
  "set_active_provider",
];

function App() {
  return (
    <main className="app-shell">
      <section className="hero-card">
        <p className="eyebrow">Tauri Desktop</p>
        <h1>CLIManager</h1>
        <p className="lead">
          Provider CRUD and local settings commands are already registered in the
          Rust backend. This placeholder removes the dead scaffold action and
          keeps the window stable until the real management UI is wired in.
        </p>

        <div className="status-row">
          <div className="status-pill">
            <span className="status-label">Frontend</span>
            <strong>No dead command calls</strong>
          </div>
          <div className="status-pill">
            <span className="status-label">Backend</span>
            <strong>Provider commands available</strong>
          </div>
        </div>
      </section>

      <section className="command-panel" aria-labelledby="command-list-title">
        <div className="panel-header">
          <p className="eyebrow">Registered Commands</p>
          <h2 id="command-list-title">Ready for the provider UI</h2>
        </div>

        <div className="command-grid">
          {backendCommands.map((command) => (
            <article className="command-card" key={command}>
              <code>{command}</code>
            </article>
          ))}
        </div>
      </section>
    </main>
  );
}

export default App;
