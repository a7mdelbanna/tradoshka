import { PublicNav } from "@/components/layout/PublicNav";

export default function PublicLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-slate-950">
      <PublicNav />
      <main className="max-w-7xl mx-auto px-6 py-8">{children}</main>
    </div>
  );
}
