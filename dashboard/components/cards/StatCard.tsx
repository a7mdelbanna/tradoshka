interface StatCardProps {
  title: string;
  value: string;
  subtitle?: string;
  variant?: "default" | "positive" | "negative";
  icon?: string;
}

export function StatCard({ title, value, subtitle, variant = "default", icon }: StatCardProps) {
  const styles = {
    default: {
      border: "border-slate-700/50",
      bg: "bg-slate-900/80",
      glow: "",
      value: "text-white",
      icon: "text-slate-400",
    },
    positive: {
      border: "border-emerald-500/20",
      bg: "bg-gradient-to-br from-slate-900 to-emerald-950/30",
      glow: "shadow-[0_0_15px_rgba(52,211,153,0.05)]",
      value: "text-emerald-400",
      icon: "text-emerald-400/60",
    },
    negative: {
      border: "border-red-500/20",
      bg: "bg-gradient-to-br from-slate-900 to-red-950/20",
      glow: "shadow-[0_0_15px_rgba(248,113,113,0.05)]",
      value: "text-red-400",
      icon: "text-red-400/60",
    },
  }[variant];

  return (
    <div className={`${styles.bg} ${styles.border} ${styles.glow} border rounded-2xl p-5 backdrop-blur-sm hover:border-slate-600/50 transition-all duration-300`}>
      <div className="flex items-center justify-between mb-3">
        <p className="text-xs font-medium uppercase tracking-wider text-slate-500">{title}</p>
        {icon && <span className={`text-lg ${styles.icon}`}>{icon}</span>}
      </div>
      <p className={`text-3xl font-bold tracking-tight ${styles.value}`}>{value}</p>
      {subtitle && <p className="text-xs text-slate-500 mt-2">{subtitle}</p>}
    </div>
  );
}
