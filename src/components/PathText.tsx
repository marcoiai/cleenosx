interface PathTextProps {
  path: string;
  className?: string;
}

export function PathText({ path, className = "" }: PathTextProps) {
  return (
    <span className={`block min-w-0 truncate font-mono text-xs ${className}`} title={path}>
      {path}
    </span>
  );
}
