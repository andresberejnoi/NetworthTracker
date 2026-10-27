import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createUser, listUsers, type UserRole } from "../../api/client";

const ROLES: UserRole[] = ["owner", "editor", "viewer"];

export default function UsersView() {
  const queryClient = useQueryClient();
  const users = useQuery({ queryKey: ["users"], queryFn: listUsers });

  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<UserRole>("viewer");

  const create = useMutation({
    mutationFn: () =>
      createUser({
        username,
        password,
        display_name: displayName.trim() || null,
        role,
      }),
    onSuccess: () => {
      setUsername("");
      setDisplayName("");
      setPassword("");
      queryClient.invalidateQueries({ queryKey: ["users"] });
    },
  });

  return (
    <section className="card" aria-label="users">
      <h2>Household members</h2>
      <p className="muted">
        Owners manage members. Editors can change data; viewers are read-only.
      </p>

      <form
        className="row"
        onSubmit={(event) => {
          event.preventDefault();
          create.mutate();
        }}
      >
        <label>
          <span>Username</span>
          <input
            aria-label="new username"
            value={username}
            onChange={(event) => setUsername(event.target.value)}
          />
        </label>
        <label>
          <span>Display name</span>
          <input
            aria-label="new display name"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
            placeholder="optional"
          />
        </label>
        <label>
          <span>Password</span>
          <input
            aria-label="new password"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
        </label>
        <label>
          <span>Role</span>
          <select
            aria-label="new role"
            value={role}
            onChange={(event) => setRole(event.target.value as UserRole)}
          >
            {ROLES.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        </label>
        <button
          type="submit"
          disabled={
            create.isPending ||
            username.trim() === "" ||
            password.length < 8
          }
        >
          Add member
        </button>
      </form>

      {create.isError && (
        <p className="error" role="alert">
          {(create.error as Error).message}
        </p>
      )}

      {users.isPending && <p className="muted">Loading members …</p>}
      {users.isError && (
        <p className="error" role="alert">
          {(users.error as Error).message}
        </p>
      )}

      {users.data && (
        <table className="table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Username</th>
              <th>Role</th>
            </tr>
          </thead>
          <tbody>
            {users.data.map((user) => (
              <tr key={user.id}>
                <td>{user.display_name}</td>
                <td>{user.username}</td>
                <td>{user.role}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
