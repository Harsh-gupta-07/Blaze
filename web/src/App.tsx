import { useEffect, useState } from "react";

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

type AppProps = {
  invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
};

export default function App({ invoke }: AppProps) {
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [path, setPath] = useState<string>("/users");

  useEffect(() => {
    let alive = true;

    setError(null);
    setFiles([]);

    invoke<FileEntry[]>("fetch_dir", {"path": path})
      .then((result) => {
        if (alive) {
          console.log(path,result)
          setFiles(result);
        }
      })
      .catch((err: unknown) => {
        if (alive) {
          setError(err instanceof Error ? err.message : String(err));
        }
      });

    return () => {
      alive = false;
    };
  }, [invoke, path]);

  function handle(file: FileEntry) {
    if (file.kind === "dir") {
      setPath(file.path);
    }
  }

  return (
    <main className="relative min-h-screen overflow-hidden bg-[#050b14] text-slate-100">
      <div className="relative mx-auto min-h-screen w-full max-w-5xl px-6 py-10 sm:px-8">
        <div className="mb-8">
          <p className="text-xs uppercase tracking-[0.45em] text-cyan-200/70" onClick={()=>setPath("/")}>
            BlazeFind
          </p>
          <h1 className="mt-3 text-4xl font-semibold tracking-tight text-white sm:text-5xl">
            Files
          </h1>
          <p className="mt-3 text-sm text-slate-400">
            {files.length
              ? `Loaded ${files.length} rows from the local database.`
              : "No rows loaded yet."}
          </p>
          {error ? (
            <p className="mt-4 rounded-2xl border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-100">
              {error}
            </p>
          ) : null}
        </div>

        <div className="grid gap-4">
          {files.map((file) => (
            <button
              key={file.id}
              className="rounded-3xl border border-white/10 bg-white/5 p-5 backdrop-blur cursor-pointer"
              onClick={()=>{handle(file)}}
            >
              <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div className="min-w-0">
                  <p className="truncate text-lg font-semibold text-white">
                    {file.name}
                  </p>
                  <p className="mt-1 break-all text-sm text-slate-400">
                    {file.path}
                  </p>
                </div>
                <span className="shrink-0 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-semibold uppercase tracking-[0.3em] text-slate-200">
                  {file.kind}
                </span>
              </div>

              <dl className="mt-4 grid gap-3 text-sm text-slate-300 sm:grid-cols-4">
                <div>
                  <dt className="text-xs uppercase tracking-[0.35em] text-slate-500">
                    Parent
                  </dt>
                  <dd className="mt-1 break-all">{file.parent || "/"}</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-[0.35em] text-slate-500">
                    Size
                  </dt>
                  <dd className="mt-1">{file.size ?? "unknown"}</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-[0.35em] text-slate-500">
                    Modified
                  </dt>
                  <dd className="mt-1">{file.modified ?? "unknown"}</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-[0.35em] text-slate-500">
                    Indexed
                  </dt>
                  <dd className="mt-1">{file.indexed}</dd>
                </div>
              </dl>
            </button>
          ))}
        </div>
      </div>
    </main>
  );
}
