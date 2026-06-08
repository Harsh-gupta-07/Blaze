import { useEffect, useState } from "react";

type AppProps = {
  invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
};

export default function App({ invoke }: AppProps) {
  const [status, setStatus] = useState<Record<string, boolean>>({});

  useEffect(() => {
    invoke<Record<string, boolean>>("daemon_status")
      .then((data) => {
        setStatus(data);
      })

      .catch(console.error);
  }, []);

  function handle() {
    invoke<boolean>("start_daemon_service")
      .then(() => {
        setStatus({});
      })
      .catch(console.error);

    setInterval(() => {
      invoke<Record<string, boolean>>("daemon_status")
        .then((data) => {
          setStatus(data);
        })

        .catch(console.error);
    }, 100);
  }

  return (
    <main>
      <h1>Daemon Status</h1>

      {Object.entries(status)
  .sort(([a], [b]) => a.localeCompare(b))
  .map(([key, value]) => (
    <div key={key}>
      {key}: {value ? "✅" : "❌"}
    </div>
))}

      <button onClick={handle}>start</button>
    </main>
  );
}
