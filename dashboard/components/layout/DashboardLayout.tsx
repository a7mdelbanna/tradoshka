import { Sidebar } from "./Sidebar";
import { Suspense } from "react";

export function DashboardLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-screen bg-slate-950">
      <Suspense>
        <Sidebar />
      </Suspense>
      <main className="flex-1 min-w-0">{children}</main>
    </div>
  );
}
