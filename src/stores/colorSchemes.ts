console.log('Loading module:', 'colorSchemes.ts');

import { emit } from "@tauri-apps/api/event";
import type { Colors, ColorScheme } from './types';
import { 
  applyColors, defaultSchemeNames, defaultColorSchemes
} from './colors';
import { preferencesStore } from './preferences';
import { createLocalStore } from './utils';
import { get } from 'svelte/store';

// Create the color schemes store using the centralized default schemes
export const colorSchemesStore = createLocalStore<ColorScheme[]>('colorSchemes', defaultColorSchemes);

/**
 * Sync default schemes - adds any missing default schemes without removing custom ones
 */
function syncDefaultSchemes(): void {
  try {
    const currentSchemes = get(colorSchemesStore);
    const currentSchemeNames = currentSchemes.map(scheme => scheme.name);
    
    // Find missing default schemes
    const missingDefaults = defaultColorSchemes.filter(
      defaultScheme => !currentSchemeNames.includes(defaultScheme.name)
    );
    
    if (missingDefaults.length > 0) {
      console.log(`Adding ${missingDefaults.length} missing default schemes:`, 
                  missingDefaults.map(s => s.name));
      
      // Add missing defaults while preserving custom schemes
      colorSchemesStore.update(schemes => [...schemes, ...missingDefaults]);
    }
  } catch (error) {
    console.error('Error syncing default schemes:', error);
  }
}

// Sync default schemes on module load
syncDefaultSchemes();

/**
 * Get all available color schemes
 */
export function getColorSchemes(): ColorScheme[] {
  return get(colorSchemesStore);
}

/**
 * Get a specific color scheme by name
 */
export function getColorScheme(name: string): ColorScheme | undefined {
  const schemes = get(colorSchemesStore);
  return schemes.find(scheme => scheme.name === name);
}

/**
 * Check if a color scheme name already exists
 */
export function colorSchemeExists(name: string): boolean {
  const schemes = get(colorSchemesStore);
  return schemes.some(scheme => scheme.name.toLowerCase() === name.toLowerCase());
}

/**
 * Save a new color scheme or update an existing one
 */
export async function saveColorScheme(name: string, colors?: Colors): Promise<string | null> {
  const trimmedName = name?.trim();
  
  if (!trimmedName) {
    console.warn("Color scheme name cannot be empty");
    return null;
  }

  // Prevent overwriting default color schemes
  const isDefaultScheme = defaultSchemeNames.some(
    name => name.toLowerCase() === trimmedName.toLowerCase()
  );
  
  if (isDefaultScheme) {
    console.warn(`Cannot overwrite default color scheme: ${trimmedName}`);
    return null;
  }

  // Use provided colors or get current colors from preferences
  const colorsToSave = colors || get(preferencesStore).colors;
  
  if (!colorsToSave) {
    console.warn("No colors available to save");
    return null;
  }

  const schemes = get(colorSchemesStore);
  const existingIndex = schemes.findIndex(
    scheme => scheme.name.toLowerCase() === trimmedName.toLowerCase()
  );

  if (existingIndex !== -1) {
    // Update existing scheme
    colorSchemesStore.update(schemes => {
      schemes[existingIndex].colors = { ...colorsToSave };
      return [...schemes];
    });
    console.log(`Color scheme updated: ${trimmedName}`);
  } else {
    // Create new scheme
    const newScheme: ColorScheme = {
      name: trimmedName,
      colors: { ...colorsToSave }
    };
    
    colorSchemesStore.update(schemes => [...schemes, newScheme]);
    console.log(`Color scheme saved: ${trimmedName}`);
  }

  // Emit event to notify other windows
  try {
    await emit('color-scheme-saved', { 
      name: trimmedName,
      colors: colorsToSave
    });
  } catch (error) {
    console.error('Error emitting color-scheme-saved event:', error);
  }

  return trimmedName;
}

/**
 * Load and apply a color scheme
 */
export async function loadColorScheme(name: string): Promise<boolean> {
  if (!name) {
    console.warn("No color scheme name provided");
    return false;
  }

  const scheme = getColorScheme(name);
  if (!scheme) {
    console.warn(`Color scheme not found: ${name}`);
    return false;
  }

  try {
    // Update preferences store with the new colors
    preferencesStore.update(prefs => ({
      ...prefs,
      colors: { ...scheme.colors }
    }));

    // Apply the colors to the current window
    await applyColors(scheme.colors);

    console.log(`Color scheme loaded: ${name}`);

    // Emit event to notify other windows
    try {
      await emit('color-scheme-loaded', {
        name: scheme.name,
        colors: scheme.colors
      });
    } catch (error) {
      console.error('Error emitting color-scheme-loaded event:', error);
    }

    return true;
  } catch (error) {
    console.error(`Error loading color scheme "${name}":`, error);
    return false;
  }
}

/**
 * Delete a custom color scheme (cannot delete default schemes)
 */
export async function deleteColorScheme(name: string): Promise<boolean> {
  if (!name) {
    console.warn("No color scheme name provided");
    return false;
  }

  // Check if it's a default scheme
  const isDefaultScheme = defaultSchemeNames.some(
    defaultName => defaultName.toLowerCase() === name.toLowerCase()
  );
  
  if (isDefaultScheme) {
    console.warn(`Cannot delete default color scheme: ${name}`);
    return false;
  }

  const schemes = get(colorSchemesStore);
  const schemeIndex = schemes.findIndex(
    scheme => scheme.name.toLowerCase() === name.toLowerCase()
  );

  if (schemeIndex === -1) {
    console.warn(`Color scheme not found: ${name}`);
    return false;
  }

  try {
    // Remove the scheme
    colorSchemesStore.update(schemes => 
      schemes.filter((_, index) => index !== schemeIndex)
    );

    console.log(`Color scheme deleted: ${name}`);

    // Emit event to notify other windows
    try {
      await emit('color-scheme-deleted', { name });
    } catch (error) {
      console.error('Error emitting color-scheme-deleted event:', error);
    }

    return true;
  } catch (error) {
    console.error(`Error deleting color scheme "${name}":`, error);
    return false;
  }
}

/**
 * Duplicate an existing color scheme with a new name
 */
export async function duplicateColorScheme(originalName: string, newName: string): Promise<string | null> {
  const trimmedNewName = newName?.trim();
  
  if (!trimmedNewName) {
    console.warn("New color scheme name cannot be empty");
    return null;
  }

  const originalScheme = getColorScheme(originalName);
  if (!originalScheme) {
    console.warn(`Original color scheme not found: ${originalName}`);
    return null;
  }

  if (colorSchemeExists(trimmedNewName)) {
    console.warn(`Color scheme already exists: ${trimmedNewName}`);
    return null;
  }

  return await saveColorScheme(trimmedNewName, originalScheme.colors);
}

/**
 * Get all custom color schemes (excluding default ones)
 */
export function getCustomColorSchemes(): ColorScheme[] {
  const allSchemes = get(colorSchemesStore);
  const defaultNames = defaultSchemeNames.map(name => name.toLowerCase());
  
  return allSchemes.filter(scheme => 
    !defaultNames.includes(scheme.name.toLowerCase())
  );
}

/**
 * Get all default color schemes
 */
export function getDefaultColorSchemes(): ColorScheme[] {
  const allSchemes = get(colorSchemesStore);
  const defaultNames = defaultSchemeNames.map(name => name.toLowerCase());
  
  return allSchemes.filter(scheme => 
    defaultNames.includes(scheme.name.toLowerCase())
  );
}

/**
 * Reset all color schemes to defaults (removes all custom schemes)
 */
export async function resetColorSchemes(): Promise<void> {
  try {
    colorSchemesStore.set([...defaultColorSchemes]);
    console.log("Color schemes reset to defaults");

    // Emit event to notify other windows
    try {
      await emit('color-schemes-reset');
    } catch (error) {
      console.error('Error emitting color-schemes-reset event:', error);
    }
  } catch (error) {
    console.error("Error resetting color schemes:", error);
  }
}

/**
 * Export a color scheme to JSON string
 */
export function exportColorScheme(name: string): string | null {
  const scheme = getColorScheme(name);
  if (!scheme) {
    console.warn(`Color scheme not found: ${name}`);
    return null;
  }

  try {
    return JSON.stringify(scheme, null, 2);
  } catch (error) {
    console.error(`Error exporting color scheme "${name}":`, error);
    return null;
  }
}

/**
 * Import a color scheme from JSON string
 */
export async function importColorScheme(jsonString: string): Promise<string | null> {
  try {
    const scheme = JSON.parse(jsonString) as ColorScheme;
    
    // Validate the imported scheme
    if (!scheme.name || !scheme.colors) {
      console.warn("Invalid color scheme format");
      return null;
    }

    // Validate that all required color properties exist
    const requiredColors: (keyof Colors)[] = [
      'primaryBg', 'secondaryBg', 'textColor', 'topbarColor', 
      'accentColor', 'hoverColor', 'warningColor', 'warningHover', 'inactiveColor'
    ];

    for (const colorKey of requiredColors) {
      if (!scheme.colors[colorKey]) {
        console.warn(`Missing required color property: ${colorKey}`);
        return null;
      }
    }

    // If scheme with same name exists, add a suffix
    let finalName = scheme.name;
    let counter = 1;
    while (colorSchemeExists(finalName)) {
      finalName = `${scheme.name} (${counter})`;
      counter++;
    }

    return await saveColorScheme(finalName, scheme.colors);
  } catch (error) {
    console.error("Error importing color scheme:", error);
    return null;
  }
}

/**
 * Get color scheme names for dropdown/selection components
 */
export function getColorSchemeNames(): string[] {
  const schemes = get(colorSchemesStore);
  return schemes.map(scheme => scheme.name);
}

/**
 * Get color scheme options for UI components
 */
export function getColorSchemeOptions(): Array<{ value: string; label: string; isDefault: boolean }> {
  const schemes = get(colorSchemesStore);
  const defaultNames = defaultColorSchemes.map(scheme => scheme.name.toLowerCase());
  
  return schemes.map(scheme => ({
    value: scheme.name,
    label: scheme.name,
    isDefault: defaultNames.includes(scheme.name.toLowerCase())
  }));
}
