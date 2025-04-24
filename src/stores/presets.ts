// src/stores/presets.ts
import { writable } from 'svelte/store';
import type { Preset } from './types';
import { defaultPreferences, TJFPreferences } from './preferences';
import { 
  lightModeColors, twilightColors, draculaColors, 
  nordColors, tokyoNightColors, monokaiProColors, 
  gruvboxColors, lcarsColors 
} from './colors';
import { loadFromLocalStorage, saveToLocalStorage } from './utils';

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

const initialPresets = loadFromLocalStorage<Preset[]>('presets', defaultPresets);

export const presetsStore = writable<Preset[]>(initialPresets);

// Save to localStorage whenever updated
presetsStore.subscribe(value => {
  saveToLocalStorage('presets', JSON.stringify(value));
});