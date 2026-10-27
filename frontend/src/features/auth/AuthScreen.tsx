import { useState } from "react";
import type { FormEvent } from "react";
import {
  login,
  setAuthToken,
  setupOwner,
  type Session,
  type User,
} from "../../api/client";

interface AuthScreenProps {
  mode: "setup" | "login";
  onAuthenticated: (user: User) => void;
}

export default function AuthScreen({ mode, onAuthenticated }: AuthScreenProps) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);

    try {
      const session: Session =
        mode === "setup"
          ? await setupOwner({
              username,
              password,
              display_name: displayName.trim() || undefined,
            })
          : await login({ username, password });

      setAuthToken(session.token);
      onAuthenticated(session.user);
    } catch (authError) {
      setError((authError as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="app">
      <header className="hero">
        <h1>NetworthTracker</h1>
        <p className="subtitle">
          {mode === "setup" ? "Create the household owner" : "Sign in"}
        </p>
      </header>

      <section className="card" aria-label="authentication">
        <form className="stack" onSubmit={submit}>
          <label className="field">
            <span>Username</span>
            <input
              aria-label="username"
              autoComplete="username"
              value={username}
              onChange={(event) => setUsername(event.target.value)}
            />
          </label>

          {mode === "setup" && (
            <label className="field">
              <span>Display name</span>
              <input
                aria-label="display name"
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
                placeholder="optional"
              />
            </label>
          )}

          <label className="field">
            <span>Password</span>
            <input
              aria-label="password"
              type="password"
              autoComplete={mode === "setup" ? "new-password" : "current-password"}
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </label>

          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}

          <button type="submit" disabled={busy}>
            {busy ? "Please wait…" : mode === "setup" ? "Create owner" : "Sign in"}
          </button>
        </form>

        {mode === "setup" && (
          <p className="muted">
            The first account is the household owner. Passwords stay on this
            machine and are hashed with Argon2.
          </p>
        )}
      </section>
    </main>
  );
}
