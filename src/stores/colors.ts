console.log('Loading module:', 'colors.ts');

import { emit } from "@tauri-apps/api/event";
import type { Colors } from './types';
import { preferencesStore, } from './preferences';
import { get } from 'svelte/store';

export const colorVariables = [
  { key: "primaryBg", label: "Primary Background" },
  { key: "secondaryBg", label: "Secondary Background" },
  { key: "textColor", label: "Text Color" },
  { key: "topbarColor", label: "Topbar Color" },
  { key: "accentColor", label: "Accent Color" },
  { key: "hoverColor", label: "Hover Color" },
  { key: "warningColor", label: "Warning Color" },
  { key: "warningHover", label: "Warning Hover Color" },
  { key: "inactiveColor", label: "Inactive Color" },
];

export const defaultColors: Colors = {
    primaryBg: "#2e3a47", // Default value for primary background
    secondaryBg: "#1f2731", // Default value for secondary background
    textColor: "#ffffff", // Default text color
    topbarColor: "#FFB81C", // Default topbar color
    accentColor: "#f0a500", // Default accent color
    hoverColor: "#ffcc00", // Default hover color
    warningColor: "#b91c1c", // Default warning color
    warningHover: "#ff4013", // Default warning hover color
    inactiveColor: "#888888" // Default inactive color
};

export const lightModeColors: Colors = {
    primaryBg: "#ffffff",    // Light gray-white for main background
    secondaryBg: "#ebebeb",  // Pure white for content areas
    textColor: "#2c3e50",   // Dark slate for text - good readability
    topbarColor: "#4a90e2", // Pleasant blue for top bar
    accentColor: "#3498db", // Slightly darker blue for interactive elements
    hoverColor: "#2980b9",  // Deeper blue for hover states
    warningColor: "#e74c3c", // Soft red for warnings
    warningHover: "#c0392b", // Deeper red for warning hovers
    inactiveColor: "#bdc3c7", // Neutral gray for inactive elements
};

export const terminalColors: Colors = {
    primaryBg: "#000000", // Default value for primary background
    secondaryBg: "#232323", // Default value for secondary background
    textColor: "#00f900", // Default text color
    topbarColor: "#7a7a7a", // Default topbar color
    accentColor: "#00f900", // Default accent color
    hoverColor: "#7aff7a", // Default hover color
    warningColor: "#000000", // Default warning color
    warningHover: "#f90000", // Default warning hover color
    inactiveColor: "#7a7a7a" // Default inactive color
};

export const twilightColors: Colors = {
    primaryBg: "#2B3A67",    // Deep blue-purple, easy on eyes
    secondaryBg: "#3F4B83",  // Lighter blue-purple for contrast
    textColor: "#E5E9FF",    // Soft white with slight blue tint
    topbarColor: "#ff8c82",  // Gold for distinctive top bar
    accentColor: "#FFC145",  // Warm gold for interactive elements
    hoverColor: "#FFB347",   // Slightly darker gold for hover states
    warningColor: "#FF6B6B", // Soft coral red for warnings
    warningHover: "#FF4949", // Brighter coral for warning hovers
    inactiveColor: "#8E9AAF"  // Muted blue-gray for inactive elements
};

export const draculaColors: Colors = {
    primaryBg: "#282a36",    // Dracula background
    secondaryBg: "#44475a",  // Dracula current line/selection
    textColor: "#f8f8f2",    // Dracula foreground
    topbarColor: "#ff79c6",  // Dracula pink
    accentColor: "#bd93f9",  // Dracula purple
    hoverColor: "#8be9fd",   // Dracula cyan
    warningColor: "#ff5555", // Dracula red
    warningHover: "#ff3333", // Brighter red for hover
    inactiveColor: "#6272a4"  // Dracula comment
};

export const nordColors: Colors = {
    primaryBg: "#2e3440",    // Nord Polar Night darkest
    secondaryBg: "#3b4252",  // Nord Polar Night lighter
    textColor: "#eceff4",    // Nord Snow Storm lightest
    topbarColor: "#88c0d0",  // Nord Frost blue
    accentColor: "#81a1c1",  // Nord Frost darker blue
    hoverColor: "#5e81ac",   // Nord Frost darkest blue
    warningColor: "#bf616a", // Nord Aurora red
    warningHover: "#d08770", // Nord Aurora orange-red
    inactiveColor: "#4c566a"  // Nord Polar Night lightest
};

export const oneDarkColors: Colors = {
    primaryBg: "#282c34",    // One Dark background
    secondaryBg: "#21252b",  // One Dark darker background
    textColor: "#abb2bf",    // One Dark foreground
    topbarColor: "#c678dd",  // One Dark purple
    accentColor: "#61afef",  // One Dark blue
    hoverColor: "#56b6c2",   // One Dark cyan
    warningColor: "#e06c75", // One Dark red
    warningHover: "#be5046", // One Dark dark red
    inactiveColor: "#5c6370"  // One Dark gray
};

export const tokyoNightColors: Colors = {
    primaryBg: "#1a1b26",    // Tokyo Night background
    secondaryBg: "#24283b",  // Tokyo Night darker background
    textColor: "#a9b1d6",    // Tokyo Night foreground
    topbarColor: "#bb9af7",  // Tokyo Night purple
    accentColor: "#7aa2f7",  // Tokyo Night blue
    hoverColor: "#2ac3de",   // Tokyo Night cyan
    warningColor: "#f7768e", // Tokyo Night red
    warningHover: "#db4b4b", // Tokyo Night dark red
    inactiveColor: "#565f89"  // Tokyo Night gray
};

export const monokaiProColors: Colors = {
    primaryBg: "#2d2a2e",    // Monokai Pro background
    secondaryBg: "#363537",  // Monokai Pro lighter bg
    textColor: "#fcfcfa",    // Monokai Pro foreground
    topbarColor: "#ff6188",  // Monokai Pro pink
    accentColor: "#78dce8",  // Monokai Pro cyan
    hoverColor: "#a9dc76",   // Monokai Pro green
    warningColor: "#fc9867", // Monokai Pro orange
    warningHover: "#ff6188", // Monokai Pro red
    inactiveColor: "#727072"  // Monokai Pro gray
};

export const gruvboxColors: Colors = {
    primaryBg: "#282828",    // Gruvbox dark background
    secondaryBg: "#3c3836",  // Gruvbox dark1
    textColor: "#ebdbb2",    // Gruvbox light0
    topbarColor: "#98971a",  // Gruvbox green (changed from yellow)
    accentColor: "#458588",  // Gruvbox blue
    hoverColor: "#d79921",   // Gruvbox yellow
    warningColor: "#cc241d", // Gruvbox red
    warningHover: "#fb4934", // Gruvbox bright red
    inactiveColor: "#928374"  // Gruvbox gray
};

export const lcarsColors: Colors = {
    primaryBg: "#000000",
    secondaryBg: "#1b1b1b",
    textColor: "#FFCC33",
    topbarColor: "#FF9966",
    accentColor: "#CC6699",
    hoverColor: "#6699CC",
    warningColor: "#FF6666",
    warningHover: "#CC3333",
    inactiveColor: "#666666",
};

// Create a collection of all available color themes for easy access
export const colorThemes = {
  default: defaultColors,
  lightMode: lightModeColors,
  terminal: terminalColors,
  twilight: twilightColors,
  dracula: draculaColors,
  nord: nordColors,
  oneDark: oneDarkColors,
  tokyoNight: tokyoNightColors,
  monokaiPro: monokaiProColors,
  gruvbox: gruvboxColors,
  lcars: lcarsColors
};

// List of theme names and labels for UI display
export const themeOptions = [
  { value: 'default', label: 'Default' },
  { value: 'lightMode', label: 'Light Mode' },
  { value: 'terminal', label: 'Terminal' },
  { value: 'twilight', label: 'Twilight' },
  { value: 'dracula', label: 'Dracula' },
  { value: 'nord', label: 'Nord' },
  { value: 'oneDark', label: 'One Dark' },
  { value: 'tokyoNight', label: 'Tokyo Night' },
  { value: 'monokaiPro', label: 'Monokai Pro' },
  { value: 'gruvbox', label: 'Gruvbox' },
  { value: 'lcars', label: 'LCARS' }
];

// Enhanced apply colors function
export async function applyColors(colors: Colors): Promise<void> {
  try {
    // Check if we've received the nested format or direct colors object
    
    if (!colors) return;
    console.log("Applying colors:", Object.keys(colors));
    
    // Apply each color with individual error handling
    Object.entries(colors).forEach(([key, value]) => {
      try {
        const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
        document.documentElement.style.setProperty(cssVariable, value);
      } catch (err) {
        console.error(`Error applying color ${key}:`, err);
      }
    });
    
  } catch (error) {
    console.error("Error in applyColors:", error);
  }
}

// Function to apply font size from preferences
export async function applyFontSize(fontSize: number): Promise<void> {
  try {
    if (!fontSize) return;
    console.log("Applying font size:", fontSize);
    
    // Set CSS variable for font size
    document.documentElement.style.setProperty('--font-size', `${fontSize}px`);
    
    // Update derived font size variables
    document.documentElement.style.setProperty('--font-size-xs', `${fontSize - 4}px`);
    document.documentElement.style.setProperty('--font-size-sm', `${fontSize - 3}px`);
    document.documentElement.style.setProperty('--font-size-md', `${fontSize - 2}px`);
    document.documentElement.style.setProperty('--font-size-lg', `${fontSize + 2}px`);
    document.documentElement.style.setProperty('--font-size-xl', `${fontSize + 8}px`);
    
    // Emit event to update other windows
    await emit("font-size-updated", { fontSize });
  } catch (error) {
    console.error("Error applying font size:", error);
  }
}

// Function to apply a specific theme by name
export async function applyTheme(themeName: keyof typeof colorThemes): Promise<void> {
  try {
    const theme = colorThemes[themeName];
    if (!theme) {
      console.error(`Theme "${themeName}" not found`);
      return;
    }
    
    // Update preferences store with the new theme
    preferencesStore.update(prefs => ({
      ...prefs,
      colors: theme
    }));
    
    // Apply the colors to the current window
    await applyColors(theme);
    
    console.log(`Applied theme: ${themeName}`);
  } catch (error) {
    console.error(`Error applying theme "${themeName}":`, error);
  }
}

// Function to change a single color
export async function changeColor(colorKey: keyof Colors, newColor: string): Promise<void> {
  try {
    // Update the color in preferencesStore
    preferencesStore.update((prefs) => {
      const updatedColors = { ...prefs.colors, [colorKey]: newColor };
      return { ...prefs, colors: updatedColors };
    });

    // Get the CSS variable name based on colorKey
    const cssVariable = `--${colorKey.replace(/([A-Z])/g, "-$1").toLowerCase()}`;

    // Update the CSS variable in the document
    document.documentElement.style.setProperty(cssVariable, newColor);

    // Special handling for text color to ensure it propagates properly
    if (colorKey === "textColor" && document.body) {
      // Apply text color to body as a fallback
      document.body.style.color = newColor;

      // Force refresh of text color on key elements
      const elements = document.querySelectorAll(
        ".color-label, p, h1, h2, h3, h4, h5, h6, span, label, button"
      );
      
      if (elements.length > 0) {
        elements.forEach((el) => {
          (el as HTMLElement).style.color = ""; // Clear explicit colors
        });
      }
    }

    // Emit event to update other windows
    await emit("color-updated", { colorKey, cssVariable, newColor });

    console.log(`Updated ${colorKey} (${cssVariable}) to ${newColor}`);
  } catch (error) {
    console.error(`Error changing color ${colorKey}:`, error);
  }
}

// Function to reset colors to defaults
export async function resetColors(): Promise<void> {
  try {
    // Update store with default colors
    preferencesStore.update((prefs) => {
      return { ...prefs, colors: defaultColors };
    });

    // Update all CSS variables
    await applyColors(defaultColors);
    
    // Emit events for each color to update other windows
    for (const [colorKey, colorValue] of Object.entries(defaultColors)) {
      const cssVariable = `--${colorKey.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
      await emit("color-updated", { colorKey, cssVariable, newColor: colorValue });
    }
    
    console.log("Reset colors to defaults");
  } catch (error) {
    console.error("Error resetting colors:", error);
  }
}

