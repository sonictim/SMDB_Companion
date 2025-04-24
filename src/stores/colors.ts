// src/stores/colors.ts
import type { Colors } from './types';

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


export function applyColorsToDocument(colors: Colors): void {
  Object.entries(colors).forEach(([key, value]) => {
    const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
    document.documentElement.style.setProperty(cssVariable, value);
  });
}