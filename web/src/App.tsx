const metrics = [
  {
    label: "Shell",
    value: "Tauri",
    note: "Desktop wrapper with a Rust backend",
  },
  {
    label: "UI",
    value: "React",
    note: "Component-driven interface layer",
  },
  {
    label: "Styling",
    value: "Tailwind",
    note: "Utility-first design system",
  },
];

const lanes = [
  {
    name: "Index health",
    state: "Ready",
    detail: "SQLite and Tantivy boot paths are wired into the project.",
  },
  {
    name: "File watcher",
    state: "Live",
    detail: "The daemon crate can keep the search index in sync.",
  },
  {
    name: "Frontend route",
    state: "Connected",
    detail: "Vite serves the app and Tauri loads the built assets.",
  },
];

export default function App() {
  return (
    <main className="relative min-h-screen overflow-hidden bg-[#050b14] text-slate-100">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,_rgba(45,212,191,0.22),_transparent_30%),radial-gradient(circle_at_top_right,_rgba(245,158,11,0.18),_transparent_24%),linear-gradient(180deg,_rgba(8,15,29,0.96),_rgba(3,7,18,1))]" />
      <div className="pointer-events-none absolute left-[-8rem] top-10 h-72 w-72 animate-[drift_10s_ease-in-out_infinite] rounded-full bg-cyan-400/10 blur-3xl" />
      <div className="pointer-events-none absolute right-[-6rem] top-40 h-80 w-80 animate-[drift_12s_ease-in-out_infinite] rounded-full bg-amber-300/10 blur-3xl" />

      <div className="relative mx-auto flex min-h-screen w-full max-w-7xl flex-col px-6 py-6 sm:px-10 lg:px-12">
        <header className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex size-11 items-center justify-center rounded-2xl bg-gradient-to-br from-cyan-300 via-teal-300 to-amber-300 text-sm font-black uppercase tracking-[0.35em] text-slate-950 shadow-lg shadow-cyan-400/20">
              B
            </div>
            <div>
              <p className="text-xs uppercase tracking-[0.45em] text-cyan-200/70">
                BlazeFind
              </p>
              <p className="text-sm text-slate-400">
                Tauri desktop shell
              </p>
            </div>
          </div>

          <div className="hidden items-center gap-3 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-[11px] uppercase tracking-[0.35em] text-slate-300 backdrop-blur md:flex">
            React
            <span className="h-1.5 w-1.5 rounded-full bg-cyan-300" />
            Tailwind
            <span className="h-1.5 w-1.5 rounded-full bg-amber-300" />
            Rust
          </div>
        </header>

        <section className="grid flex-1 items-center gap-8 py-10 lg:grid-cols-[1.15fr_0.85fr] lg:py-14">
          <div className="space-y-8">
            <div className="inline-flex rounded-full border border-cyan-300/20 bg-cyan-300/10 px-4 py-2 text-xs uppercase tracking-[0.35em] text-cyan-100">
              Desktop search workspace
            </div>

            <div className="space-y-6">
              <h1 className="max-w-3xl text-5xl font-semibold tracking-tight text-white sm:text-6xl lg:text-7xl">
                A focused Tauri app shell for search, indexing, and command flow.
              </h1>
              <p className="max-w-2xl text-base leading-7 text-slate-300 sm:text-lg">
                The Rust backend, React frontend, and Tailwind styling are now
                scaffolded together. This gives you a clean starting point for a
                desktop product without fighting the stack.
              </p>
            </div>

            <div className="flex flex-wrap gap-3">
              <button className="rounded-full bg-white px-5 py-3 text-sm font-semibold text-slate-950 shadow-lg shadow-white/10 transition hover:-translate-y-0.5 hover:bg-cyan-100">
                Open command palette
              </button>
              <button className="rounded-full border border-white/10 bg-white/5 px-5 py-3 text-sm font-semibold text-slate-100 backdrop-blur transition hover:-translate-y-0.5 hover:bg-white/10">
                Inspect pipeline
              </button>
            </div>

            <div className="grid gap-4 sm:grid-cols-3">
              {metrics.map((metric) => (
                <article
                  key={metric.label}
                  className="rounded-3xl border border-white/10 bg-white/5 p-5 backdrop-blur"
                >
                  <p className="text-xs uppercase tracking-[0.35em] text-slate-400">
                    {metric.label}
                  </p>
                  <p className="mt-4 text-2xl font-semibold text-white">
                    {metric.value}
                  </p>
                  <p className="mt-2 text-sm leading-6 text-slate-400">
                    {metric.note}
                  </p>
                </article>
              ))}
            </div>
          </div>

          <div className="relative">
            <div className="absolute inset-0 -rotate-6 rounded-[2rem] bg-gradient-to-br from-cyan-400/20 via-transparent to-amber-400/20 blur-3xl" />

            <div className="relative rounded-[2rem] border border-white/10 bg-slate-950/70 p-6 shadow-2xl shadow-black/40 backdrop-blur">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm uppercase tracking-[0.35em] text-slate-500">
                    Search surface
                  </p>
                  <h2 className="mt-2 text-2xl font-semibold text-white">
                    Ready for commands
                  </h2>
                </div>
                <div className="rounded-full border border-emerald-300/20 bg-emerald-300/10 px-3 py-1 text-xs font-semibold uppercase tracking-[0.3em] text-emerald-200">
                  Live
                </div>
              </div>

              <div className="mt-6 rounded-3xl border border-white/10 bg-slate-900/80 p-5">
                <div className="flex items-center gap-3 text-sm text-slate-400">
                  <span className="rounded-full border border-cyan-300/20 bg-cyan-300/10 px-3 py-1 text-xs font-semibold uppercase tracking-[0.3em] text-cyan-100">
                    Cmd
                  </span>
                  Type to filter files, commands, or snippets.
                </div>

                <div className="mt-5 grid gap-3">
                  {lanes.map((lane) => (
                    <div
                      key={lane.name}
                      className="flex items-center justify-between gap-4 rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-4"
                    >
                      <div>
                        <p className="font-medium text-white">{lane.name}</p>
                        <p className="mt-1 text-sm leading-6 text-slate-400">
                          {lane.detail}
                        </p>
                      </div>
                      <span className="shrink-0 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-semibold uppercase tracking-[0.3em] text-slate-200">
                        {lane.state}
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              <div className="mt-6 grid gap-3 sm:grid-cols-2">
                <div className="rounded-2xl border border-white/10 bg-white/5 p-4">
                  <p className="text-xs uppercase tracking-[0.35em] text-slate-500">
                    Backend
                  </p>
                  <p className="mt-3 text-lg font-semibold text-white">
                    Rust services
                  </p>
                  <p className="mt-2 text-sm leading-6 text-slate-400">
                    The existing workspace crates remain available for indexing
                    and file-system work.
                  </p>
                </div>
                <div className="rounded-2xl border border-white/10 bg-gradient-to-br from-cyan-300/10 to-amber-300/10 p-4">
                  <p className="text-xs uppercase tracking-[0.35em] text-slate-500">
                    Frontend
                  </p>
                  <p className="mt-3 text-lg font-semibold text-white">
                    React + Tailwind
                  </p>
                  <p className="mt-2 text-sm leading-6 text-slate-400">
                    The UI is isolated in `web/` so it can iterate independently
                    from the Rust code.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </section>

        <footer className="flex flex-col gap-2 border-t border-white/10 py-6 text-sm text-slate-500 sm:flex-row sm:items-center sm:justify-between">
          <p>Built on top of the existing Rust workspace.</p>
          <p>Frontend in `web/`, Tauri backend in `src-tauri/`.</p>
        </footer>
      </div>
    </main>
  );
}
