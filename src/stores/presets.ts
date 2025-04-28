console.log('Loading module:', 'presets.ts');  // Add to each file
import type { Preset } from './types';
import { defaultPreferences, TJFPreferences } from './preferences';
import { 
  lightModeColors, twilightColors, draculaColors, 
  nordColors, tokyoNightColors, monokaiProColors, 
  gruvboxColors, lcarsColors 
} from './colors';
import { createLocalStore } from './utils';
import { get } from 'svelte/store';
import {preferencesStore} from './preferences';
import { applyColors } from './colors';
import { invoke } from "@tauri-apps/api/core";
import { emit } from '@tauri-apps/api/event';

const defaultPresets: Preset[] = [
  { name: "Default", pref: defaultPreferences },
  { name: "TJF", pref: TJFPreferences },
  { name: "Light Mode", pref: { ...defaultPreferences, colors: lightModeColors } },
  { name: "Twilight", pref: { ...defaultPreferences, colors: twilightColors } },
  { name: "Dracula", pref: { ...defaultPreferences, colors: draculaColors } },
  { name: "Nord", pref: { ...defaultPreferences, colors: nordColors } },
  { name: "Tokyo Night", pref: { ...defaultPreferences, colors: tokyoNightColors } },
  { name: "Monokai Pro", pref: { ...defaultPreferences, colors: monokaiProColors } },
  { name: "Gruvbox", pref: { ...defaultPreferences, colors: gruvboxColors } },
  { name: "LCARS", pref: { ...defaultPreferences, colors: lcarsColors } },
];

// Export the store if you need to access the list of presets elsewhere
// For example, to display a dropdown of available presets
export const presetsStore = createLocalStore<Preset[]>('presets', defaultPresets);

export async function applyPreset(preset: Preset)  {
    if (!preset || !preset.pref) {
        console.warn("Invalid preset provided.");
        return;
    }
    preferencesStore.set(preset.pref);
    await applyColors(preset.pref.colors);
    console.log("Applied preset:", preset.name);

}



export async function loadPreset(name: string | null) {
    if (!name) {
        console.warn("No preset name provided.");
        return;
    }
  console.log("Loading preset:", name);
  
  const presets = get(presetsStore);
  const preset = presets.find(p => p.name === name);
  
  if (preset) {
    preferencesStore.set(preset.pref);
    
    emit('preset-change', { 
        preset: preset
    }).catch(err => {
      console.error('Error emitting preset-change event:', err);
    });
  } else {
    console.warn(`Preset not found: ${name}`);
  }
}

export function savePreset(newPreset: string): string | undefined {
    const trimmedPreset = newPreset?.trim();
    const presets = get(presetsStore);

    // Make sure the preset name is valid
    if (trimmedPreset) {
      if (trimmedPreset === "Default") {
        console.log("Cannot update or save the Default preset.");
        return;
      }

      // Check if the preset already exists
      const existingPresetIndex = presets.findIndex(
        (p) => p.name === trimmedPreset
      );

      if (existingPresetIndex !== -1) {
        // If it exists, update its preferences
        presetsStore.update((presets) => {
          presets[existingPresetIndex].pref = get(preferencesStore); // Update the preferences
          return [...presets]; // Return updated presets
        });
        console.log("Preset updated:", trimmedPreset);
      } else {
        // If it doesn't exist, create a new preset
        presetsStore.update((presets) => [
          ...presets,
          { name: trimmedPreset, pref: get(preferencesStore) },
        ]);
        console.log("Preset saved:", trimmedPreset);
      }

      return trimmedPreset;

    }
  }

export function deletePreset(selectedPreset: string): void {
    if (!selectedPreset || selectedPreset === "Default") {
      console.log("Cannot delete the Default preset.");
      return;
    }

    // Remove the selected preset
    presetsStore.update((presets) =>
      presets.filter((p) => p.name !== selectedPreset)
    );

    console.log("Preset deleted:", selectedPreset);
    
    // Don't reassign parameter
    // Return instead of modifying the parameter
    return;
}
