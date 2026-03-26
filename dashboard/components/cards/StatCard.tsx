interface StatCardProps {
  title: string;
  value: string;
  subtitle?: string;
  variant?: "default" | "positive" | "negative";
}

export function StatCard({ title, value, subtitle, variant = "default" }: StatCardProps) {
  const valueColor = { default: "text-slate-100", positive: "text-emerald-400", negative: "text-red-400" }[variant];
  return (
    <div className="bg-slate-900 border border-slate-800 rounded-xl p-5">
      <p className="text-sm text-slate-400 mb-1">{title}</p>
      <p className={`text-2xl font-bold ${valueColor}`}>{value}</p>
      {subtitle && <p className="text-xs text-slate-500 mt-1">{subtitle}</p>}
    </div>
  );
}
