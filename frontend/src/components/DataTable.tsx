import type { ReactNode } from "react";

export default function DataTable({ children }: { children: ReactNode }) {
  return (
    <div className="table-scroll">
      <table className="table">{children}</table>
    </div>
  );
}
