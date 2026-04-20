import { cn } from '@/lib/utils';

interface StatusDotProps {
  status: 'unknown' | 'ok' | 'slow' | 'error';
  className?: string;
}

export function StatusDot({ status, className }: StatusDotProps) {
  return (
    <span
      role="status"
      aria-live="polite"
      className={cn(
        'inline-block h-2 w-2 rounded-full',
        status === 'ok' && 'bg-success',
        status === 'slow' && 'bg-warning',
        status === 'error' && 'bg-destructive',
        status === 'unknown' && 'bg-muted-foreground',
        className
      )}
    />
  );
}
