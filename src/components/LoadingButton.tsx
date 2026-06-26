import type { ButtonHTMLAttributes, ReactNode } from "react";

interface LoadingButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  loading?: boolean;
  children: ReactNode;
}

export function LoadingButton({ loading, children, className = "", disabled, ...props }: LoadingButtonProps) {
  return (
    <button
      className={`inline-flex min-h-10 items-center justify-center gap-2 rounded-lg bg-blue-700 px-4 text-sm font-semibold text-white shadow-sm transition hover:bg-blue-800 disabled:cursor-not-allowed disabled:bg-slate-400 ${className}`}
      disabled={disabled || loading}
      {...props}
    >
      {loading && <span className="h-4 w-4 animate-spin rounded-full border-2 border-white/60 border-t-white" />}
      {children}
    </button>
  );
}

