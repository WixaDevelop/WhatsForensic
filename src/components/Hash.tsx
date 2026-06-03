/**
 * Display monoespaciado de un hash SHA-256, truncado pero copiable.
 */

interface HashProps {
  value: string;
  truncate?: boolean;
}

export function Hash({ value, truncate = true }: HashProps) {
  const display = truncate && value.length > 16 ? `${value.slice(0, 8)}…${value.slice(-8)}` : value;
  return (
    <code className="hash" title={value}>
      {display}
    </code>
  );
}
