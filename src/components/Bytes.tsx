/**
 * Display de un valor en bytes en unidades binarias humanas.
 */

import { formatBytes } from '../utils/format';

interface BytesProps {
  value: number;
}

export function Bytes({ value }: BytesProps) {
  return <span>{formatBytes(value)}</span>;
}
