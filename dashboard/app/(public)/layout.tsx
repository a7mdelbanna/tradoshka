import { PublicNav } from "@/components/layout/PublicNav";

export default function PublicLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <PublicNav />
      <main className="max-w-7xl mx-auto px-6 py-8">{children}</main>
    </>
  );
}
