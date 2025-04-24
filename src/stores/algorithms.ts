import type { Algorithm } from './types';

export const defaultAlgorithms: Algorithm[] = [
  { id: 'basic', name: 'Duplicate Search', enabled: true },
  { id: 'waveform', name: 'Audio Content Comparison', enabled: false, db: null },
  { id: 'invalidpath', name: 'Invalid Files', enabled: false },
  { id: 'filename', name: 'Similar Filename', enabled: false },
  { id: 'dual_mono', name: 'Dual Mono Check', enabled: false },
  { id: 'duration', name: 'Minimum Duration:', enabled: false, min_dur: 0.5 },
  { id: 'audiosuite', name: 'Audiosuite Tags', enabled: false },
  { id: 'filetags', name: 'Filename Contains Tag', enabled: false },
  { id: 'dbcompare', name: 'Database Compare:', enabled: false },
];