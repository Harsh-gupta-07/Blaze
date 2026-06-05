import { useEffect, useRef, useState, useCallback } from "react";

/* ── Types ─────────────────────────────────────────────── */

type FileEntry = {
  id: number;
  path: string;
  parent: string;
  name: string;
  size: number | null;
  modified: number | null;
  kind: string;
  indexed: number;
};

type StartupStatus = {
  kind: "warm" | "cold";
  last_event_id: number | null;
};

type AppProps = {
  invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
};

/* ── Helpers ────────────────────────────────────────────── */

function formatBytes(bytes: number | null): string {
  if (bytes === null) return "—";
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / 1024 ** i).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function formatModified(ts: number | null): string {
  if (ts === null) return "—";
  const d = new Date(ts * 1000);
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "2-digit",
    year: "numeric",
  });
}

function getFileIcon(kind: string, name: string): string {
  if (kind === "dir") return "folder";
  if (kind === "symlink") return "link";
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    png: "image",
    jpg: "image",
    jpeg: "image",
    gif: "image",
    svg: "image",
    webp: "image",
    mp4: "movie",
    mov: "movie",
    avi: "movie",
    mkv: "movie",
    mp3: "audio_file",
    wav: "audio_file",
    flac: "audio_file",
    pdf: "picture_as_pdf",
    zip: "folder_zip",
    gz: "folder_zip",
    tar: "folder_zip",
    "7z": "folder_zip",
    js: "code",
    ts: "code",
    tsx: "code",
    jsx: "code",
    rs: "code",
    py: "code",
    go: "code",
    rb: "code",
    cpp: "code",
    c: "code",
    h: "code",
    json: "data_object",
    toml: "data_object",
    yaml: "data_object",
    xml: "data_object",
    md: "description",
    txt: "description",
    csv: "table_chart",
    html: "html",
    css: "css",
    sh: "terminal",
    bash: "terminal",
    zsh: "terminal",
    db: "database",
    sqlite: "database",
  };
  return map[ext] ?? "draft";
}

function buildBreadcrumbs(
  path: string,
): Array<{ label: string; target: string }> {
  if (path === "/" || path === "") return [{ label: "/", target: "/" }];
  const parts = path.split("/").filter(Boolean);
  const crumbs = [{ label: "/", target: "/" }];
  let accum = "";
  for (const p of parts) {
    accum += "/" + p;
    crumbs.push({ label: p, target: accum });
  }
  return crumbs;
}

/* ── StartupBanner ──────────────────────────────────────── */

function StartupBanner({ status }: { status: StartupStatus | null }) {
  if (!status) return null;

  const isWarm = status.kind === "warm";

  return (
    <div className="startup-panel animate-in" style={{ marginBottom: 0 }}>
      {/* icon */}
      <span
        className="material-symbols-outlined"
        style={{
          color: isWarm ? "var(--warm-color)" : "var(--cold-color)",
          fontSize: 20,
          marginTop: 1,
          flexShrink: 0,
        }}
      >
        {isWarm ? "cached" : "rocket_launch"}
      </span>

      {/* text */}
      <div style={{ minWidth: 0 }}>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            marginBottom: 4,
          }}
        >
          <span
            style={{
              fontFamily: "'Hanken Grotesk', sans-serif",
              fontWeight: 600,
              fontSize: 14,
              color: "var(--on-surface)",
            }}
          >
            {isWarm ? "Warm Restart" : "Cold Start"}
          </span>
          <span className={`badge ${isWarm ? "badge-warm" : "badge-cold"}`}>
            {isWarm ? "journal replay" : "full bootstrap"}
          </span>
        </div>

        <p
          style={{
            margin: 0,
            fontSize: 13,
            color: "var(--on-surface-variant)",
            lineHeight: 1.5,
          }}
        >
          {isWarm ? (
            <>
              Blaze resumed from FSEvents event ID{" "}
              <code
                className="mono"
                style={{
                  fontSize: 12,
                  color: "var(--warm-color)",
                  background: "var(--warm-bg)",
                  padding: "1px 6px",
                  borderRadius: 3,
                }}
              >
                {status.last_event_id?.toLocaleString() ?? "—"}
              </code>
              . macOS replayed all filesystem events that occurred while the app
              was offline — no full scan needed.
            </>
          ) : (
            <>
              No previous checkpoint found. Blaze performed a full filesystem
              scan and rebuilt the index from scratch. Future restarts will use{" "}
              <span style={{ color: "var(--warm-color)", fontWeight: 500 }}>
                journal replay
              </span>
              .
            </>
          )}
        </p>
      </div>

      {/* live indicator */}
      <div
        style={{
          marginLeft: "auto",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 4,
          flexShrink: 0,
        }}
      >
        <div className="status-dot live" />
        <span className="label-caps" style={{ whiteSpace: "nowrap" }}>
          live
        </span>
      </div>
    </div>
  );
}

/* ── Main App ───────────────────────────────────────────── */

export default function App({ invoke }: AppProps) {
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [path, setPath] = useState<string>(".");
  const [selected, setSelected] = useState<number | null>(null);
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState<StartupStatus | null>(null);
  const [showBanner, setShowBanner] = useState(true);
  const searchRef = useRef<HTMLInputElement>(null);

  /* Fetch startup status once on mount */
  useEffect(() => {
    invoke<StartupStatus>("get_startup_status")
      .then(setStatus)
      .catch(() => {});
  }, [invoke]);

  /* Fetch directory on path change */
  useEffect(() => {
    let alive = true;
    setError(null);
    setFiles([]);
    setSelected(null);

    invoke<FileEntry[]>("fetch_dir", { path })
      .then((result) => {
        if (alive) setFiles(result);
      })
      .catch((err: unknown) => {
        if (alive) setError(err instanceof Error ? err.message : String(err));
      });

    return () => {
      alive = false;
    };
  }, [invoke, path]);

  /* ⌘+K to focus search */
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const navigate = useCallback((f: FileEntry) => {
    if (f.kind === "dir") setPath(f.path);
    else setSelected(f.id);
  }, []);

  const crumbs = buildBreadcrumbs(path);

  const filtered = files.filter((f) =>
    search.trim() === ""
      ? true
      : f.name.toLowerCase().includes(search.toLowerCase()),
  );

  /* sidebar quick-access paths */
  const sidebarItems = [
    { icon: "home", label: "Home", target: "/Users" },
    {
      icon: "desktop_windows",
      label: "Desktop",
      target: "/Users/Harsh/Desktop",
    },
    { icon: "download", label: "Downloads", target: "/Users/Harsh/Downloads" },
    {
      icon: "description",
      label: "Documents",
      target: "/Users/Harsh/Documents",
    },
    { icon: "database", label: "System Drive", target: "/" },
  ];

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        background: "var(--surface)",
        color: "var(--on-surface)",
        fontFamily: "'Inter', sans-serif",
        overflow: "hidden",
      }}
    >
      {/* ── TopBar ─────────────────────────────────────────── */}
      <header
        style={{
          background: "var(--surface-container-low)",
          borderBottom: "1px solid var(--outline-variant)",
          height: 52,
          display: "flex",
          alignItems: "center",
          padding: "0 20px",
          gap: 16,
          flexShrink: 0,
          zIndex: 20,
        }}
      >
        {/* Brand */}
        <span
          style={{
            fontFamily: "'Hanken Grotesk', sans-serif",
            fontWeight: 700,
            fontSize: 18,
            color: "var(--on-surface)",
            letterSpacing: "-0.02em",
            flexShrink: 0,
          }}
        >
          Blaze
          <span style={{ color: "var(--primary)" }}>Find</span>
        </span>

        {/* Breadcrumbs */}
        <nav
          style={{
            display: "flex",
            alignItems: "center",
            gap: 2,
            overflow: "hidden",
            flex: "0 1 auto",
          }}
        >
          {crumbs.map((c, i) => (
            <span
              key={c.target}
              style={{ display: "flex", alignItems: "center", gap: 2 }}
            >
              {i > 0 && (
                <span
                  className="mono"
                  style={{
                    color: "var(--outline)",
                    fontSize: 12,
                    padding: "0 2px",
                  }}
                >
                  /
                </span>
              )}
              <span
                className={`breadcrumb-seg ${i === crumbs.length - 1 ? "active" : ""}`}
                onClick={() => i < crumbs.length - 1 && setPath(c.target)}
              >
                {c.label}
              </span>
            </span>
          ))}
        </nav>

        {/* Search */}
        <div
          style={{
            flex: 1,
            maxWidth: 360,
            position: "relative",
            marginLeft: "auto",
          }}
        >
          <span
            className="material-symbols-outlined"
            style={{
              position: "absolute",
              left: 6,
              top: "50%",
              transform: "translateY(-50%)",
              color: "var(--outline)",
              fontSize: 16,
            }}
          >
            search
          </span>
          <input
            ref={searchRef}
            className="search-input"
            placeholder="Filter files…  (⌘K)"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>

        {/* File count */}
        <span
          className="label-caps"
          style={{ flexShrink: 0, color: "var(--outline)", marginLeft: 8 }}
        >
          {filtered.length} items
        </span>
      </header>

      {/* ── Body ───────────────────────────────────────────── */}
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        {/* Sidebar */}
        <nav
          style={{
            width: 220,
            background: "var(--surface-container-low)",
            borderRight: "1px solid var(--outline-variant)",
            display: "flex",
            flexDirection: "column",
            padding: "16px 0",
            flexShrink: 0,
            overflowY: "auto",
          }}
        >
          <div style={{ padding: "0 20px 10px" }}>
            <span className="label-caps">Locations</span>
          </div>
          {sidebarItems.map((item) => (
            <button
              key={item.target}
              className={`nav-item ${path === item.target ? "nav-active" : ""}`}
              onClick={() => setPath(item.target)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 10,
                padding: "7px 20px 7px 22px",
                background: "none",
                border: "none",
                color:
                  path === item.target ? "var(--primary)" : "var(--outline)",
                cursor: "pointer",
                fontSize: 13,
                width: "100%",
                textAlign: "left",
                fontFamily: "'Inter', sans-serif",
              }}
            >
              <span
                className="material-symbols-outlined"
                style={{ fontSize: 16 }}
              >
                {item.icon}
              </span>
              {item.label}
            </button>
          ))}

          {/* Startup summary in sidebar */}
          {status && (
            <div
              style={{
                margin: "auto 12px 0",
                paddingTop: 16,
                borderTop: "1px solid var(--outline-variant)",
                marginTop: "auto",
              }}
            >
              <div
                style={{
                  padding: "8px 8px",
                  borderRadius: 6,
                  background: "var(--surface-container)",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    marginBottom: 5,
                  }}
                >
                  <div
                    className={`status-dot ${status.kind}`}
                    style={{ marginTop: 0 }}
                  />
                  <span className="label-caps" style={{ fontSize: 10 }}>
                    {status.kind === "warm" ? "Warm Restart" : "Cold Start"}
                  </span>
                </div>
                <p
                  style={{
                    margin: 0,
                    fontSize: 11,
                    color: "var(--outline)",
                    lineHeight: 1.4,
                    fontFamily: "'JetBrains Mono', monospace",
                  }}
                >
                  {status.kind === "warm"
                    ? `Resumed from event\n#${status.last_event_id?.toLocaleString()}`
                    : "Full scan on boot"}
                </p>
              </div>
            </div>
          )}
        </nav>

        {/* Main content */}
        <main
          style={{
            flex: 1,
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}
        >
          {/* Startup banner */}
          {showBanner && status && (
            <div style={{ padding: "12px 24px 0" }}>
              <div style={{ position: "relative" }}>
                <StartupBanner status={status} />
                <button
                  onClick={() => setShowBanner(false)}
                  style={{
                    position: "absolute",
                    top: 10,
                    right: 10,
                    background: "none",
                    border: "none",
                    color: "var(--outline)",
                    cursor: "pointer",
                    padding: 2,
                    display: "flex",
                    alignItems: "center",
                  }}
                  title="Dismiss"
                >
                  <span
                    className="material-symbols-outlined"
                    style={{ fontSize: 16 }}
                  >
                    close
                  </span>
                </button>
              </div>
            </div>
          )}

          {/* Error */}
          {error && (
            <div
              style={{
                margin: "12px 24px 0",
                padding: "10px 14px",
                background: "rgba(255,180,171,0.08)",
                border: "1px solid rgba(255,180,171,0.2)",
                borderRadius: 6,
                color: "var(--error)",
                fontSize: 13,
                display: "flex",
                alignItems: "center",
                gap: 8,
              }}
            >
              <span
                className="material-symbols-outlined"
                style={{ color: "var(--error)", fontSize: 16 }}
              >
                error
              </span>
              {error}
            </div>
          )}

          {/* Column headers */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              padding: "8px 24px",
              borderBottom: "1px solid var(--outline-variant)",
              marginTop: 12,
              flexShrink: 0,
            }}
          >
            <div style={{ flex: 1, paddingLeft: 28 }} className="label-caps">
              Name
            </div>
            <div style={{ width: 180 }} className="label-caps">
              Modified
            </div>
            <div
              style={{ width: 100, textAlign: "right" }}
              className="label-caps"
            >
              Size
            </div>
          </div>

          {/* File list */}
          <div style={{ flex: 1, overflowY: "auto" }}>
            {filtered.length === 0 && !error && (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  justifyContent: "center",
                  height: 200,
                  color: "var(--outline)",
                  gap: 8,
                }}
              >
                <span
                  className="material-symbols-outlined"
                  style={{ fontSize: 36 }}
                >
                  folder_open
                </span>
                <span style={{ fontSize: 13 }}>No files here</span>
              </div>
            )}

            {filtered.map((file) => {
              const isSelected = selected === file.id;
              const isDir = file.kind === "dir";

              return (
                <div
                  key={file.id}
                  className={`file-row ${isSelected ? "selected" : ""}`}
                  onClick={() => navigate(file)}
                  style={{
                    borderLeft: isSelected
                      ? "2px solid var(--primary)"
                      : "2px solid transparent",
                  }}
                >
                  {/* icon */}
                  <span
                    className={`material-symbols-outlined ${isSelected && !isDir ? "icon-filled" : ""}`}
                    style={{
                      color: isDir
                        ? isSelected
                          ? "var(--primary)"
                          : "var(--outline)"
                        : isSelected
                          ? "var(--primary)"
                          : "var(--outline)",
                      marginRight: 10,
                      fontSize: 18,
                      flexShrink: 0,
                    }}
                  >
                    {getFileIcon(file.kind, file.name)}
                  </span>

                  {/* name */}
                  <div
                    className="file-name"
                    style={{
                      flex: 1,
                      fontSize: 13,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                      color: isSelected
                        ? "var(--primary)"
                        : "var(--on-surface)",
                      fontWeight: isSelected ? 600 : 400,
                    }}
                  >
                    {file.name}
                    {isDir && (
                      <span
                        style={{
                          color: "var(--outline)",
                          marginLeft: 4,
                          fontSize: 11,
                        }}
                      >
                        /
                      </span>
                    )}
                  </div>

                  {/* modified */}
                  <div
                    className="file-meta mono"
                    style={{
                      width: 180,
                      fontSize: 12,
                      color: isSelected ? "var(--primary)" : "var(--outline)",
                    }}
                  >
                    {formatModified(file.modified)}
                  </div>

                  {/* size */}
                  <div
                    className="file-meta mono"
                    style={{
                      width: 100,
                      textAlign: "right",
                      fontSize: 12,
                      color: isSelected ? "var(--primary)" : "var(--outline)",
                    }}
                  >
                    {isDir ? "—" : formatBytes(file.size)}
                  </div>
                </div>
              );
            })}
          </div>
        </main>
      </div>

      {/* ── Footer ─────────────────────────────────────────── */}
      <footer
        style={{
          background: "var(--surface-container-low)",
          borderTop: "1px solid var(--outline-variant)",
          height: 28,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0 20px",
          flexShrink: 0,
          zIndex: 10,
        }}
      >
        <span
          className="mono"
          style={{ fontSize: 11, color: "var(--outline)" }}
        >
          {filtered.length} items
          {selected !== null && " · 1 selected"}
        </span>

        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          {status && (
            <span
              className={`badge ${status.kind === "warm" ? "badge-warm" : "badge-cold"}`}
              style={{ padding: "1px 8px", fontSize: 10 }}
            >
              <span
                className="material-symbols-outlined"
                style={{ fontSize: 11, marginRight: 3 }}
              >
                {status.kind === "warm" ? "cached" : "rocket_launch"}
              </span>
              {status.kind === "warm" ? "warm restart" : "cold start"}
            </span>
          )}
          <span
            className="mono"
            style={{ fontSize: 11, color: "var(--outline)" }}
          >
            ⌘K to search
          </span>
        </div>
      </footer>
    </div>
  );
}
