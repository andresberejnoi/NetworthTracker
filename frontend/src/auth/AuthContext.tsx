import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import type { User } from "../api/client";

const AuthContext = createContext<User | null>(null);

export function AuthProvider({
  user,
  children,
}: {
  user: User | null;
  children: ReactNode;
}) {
  return <AuthContext.Provider value={user}>{children}</AuthContext.Provider>;
}

/** The signed-in user, or `null` outside an authenticated tree. */
export function useCurrentUser(): User | null {
  return useContext(AuthContext);
}
