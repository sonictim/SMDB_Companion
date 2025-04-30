console.log('Loading module:', 'algorithms.ts');  // Add to each file

import type { Algorithm } from './types';
import { preferencesStore } from './preferences';

export const defaultAlgorithms: Algorithm[] = [
  { id: 'basic', name: 'Duplicate Search', enabled: true },
  { id: 'filename', name: 'Similar Filename', enabled: false },
  { id: 'audiosuite', name: 'Audiosuite Tags', enabled: false },
  { id: 'waveform', name: 'Audio Content Comparison', enabled: false, db: null },
  { id: 'dual_mono', name: 'Dual Mono Check', enabled: false },
  { id: 'filetags', name: 'Filename Contains Tag', enabled: false },
  { id: 'invalidpath', name: 'Invalid Files', enabled: false },
  { id: 'duration', name: 'Minimum Duration', enabled: false, min_dur: 0.5 },
  { id: 'dbcompare', name: 'Database Compare', enabled: false },
];

  export function getAlgoClass(algo: { id: string }, algorithms: any[]) {
    if (
      (algo.id === "audiosuite" || algo.id === "filename") &&
      !algorithms.find((a) => a.id === "basic")?.enabled
    ) {
      return "inactive";
    }
    return "";
  }

  export function toggleAlgorithm(id: string) {
    preferencesStore.update((prefs) => ({
      ...prefs,
      algorithms: prefs.algorithms.map((algo) =>
        algo.id === id ? { ...algo, enabled: !algo.enabled } : algo
      ),
    }));
  }

   export function getAlgorithmTooltip(id: string): string {
    const tooltips: Record<string, string> = {
      basic: "Finds duplicates by comparing Match Criteria set in Preferences.",
      filename:
        "Will attempt to remove extra letters and numbers (.1.4.21.M.wav) from the filename",
      audiosuite:
        "Searches for Protools Audiosuite tags in the filename and checks for orginal file.",
      duration: "Files less than the set duration will be marked for removal.",
      waveform:
        "Compares audio waveform patterns to find similar sounds.  This may take a while.",
      dbcompare: "Compares against another database to find duplicates.",
      invalidpath: "Files with invalid paths will be marked for removal.",
      filetags:
        "Filenames containting tags in this list will be marked for removal.",
      dual_mono:
        "Files where all channels contain identical audio will be identified.  User can choose to remove extra channels in results panel.",
    };

    return tooltips[id] || "No description available";
  }
