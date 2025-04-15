<script lang="ts">
  import { emit } from "@tauri-apps/api/event";
  import { preferencesStore } from "../../store";
  import { get } from "svelte/store";
  import type { Preferences, Colors } from "../../store";

  // Use the $: syntax to ensure preferences stays reactive
  $: preferences = $preferencesStore;

  async function changeColor(colorKey: keyof Colors, newColor: string) {
    // Update the color in preferencesStore
    preferencesStore.update((prefs) => {
      const updatedColors = { ...prefs.colors, [colorKey]: newColor };
      return { ...prefs, colors: updatedColors };
    });

    // Get the CSS variable name based on colorKey by converting camelCase to kebab-case
    const cssVariable = `--${colorKey.replace(/([A-Z])/g, "-$1").toLowerCase()}`;

    // Update the CSS variable in the document
    document.documentElement.style.setProperty(cssVariable, newColor);

    // Special handling for text color to ensure it propagates properly
    if (colorKey === "textColor") {
      // Apply text color to body as a fallback
      document.body.style.color = newColor;

      // Force refresh of text color on key elements
      document
        .querySelectorAll(
          ".color-label, p, h1, h2, h3, h4, h5, h6, span, label, button"
        )
        .forEach((el) => {
          (el as HTMLElement).style.color = ""; // Clear explicit colors
        });
    }

    // Emit the change to all windows
    await emit("color-updated", { colorKey, cssVariable, newColor });

    // Log the change to the console
    console.log(`Updated ${colorKey} (${cssVariable}) to ${newColor}`);
  }

  // Color variables with mapping between store keys and display labels
  const colorVariables = [
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

  // Function to get current color value - this is now reactive
  function getCurrentColor(colorKey: keyof Colors): string {
    return preferences?.colors[colorKey] || "";
  }

  // Function to reset colors to defaults
  function resetColors() {
    // Get the default preferences from your store
    const defaultColors = {
      primaryBg: "#2e3a47", // Default value for primary background
      secondaryBg: "#1f2731", // Default value for secondary background
      textColor: "#ffffff", // Default text color
      topbarColor: "#FFB81C", // Default topbar color
      accentColor: "#f0a500", // Default accent color
      hoverColor: "#ffcc00", // Default hover color
      warningColor: "#b91c1c", // Default warning color
      warningHover: "#dc2626", // Default warning hover color
      inactiveColor: "#888888", // Default inactive color
    };

    // Update store with default colors
    preferencesStore.update((prefs) => {
      return { ...prefs, colors: defaultColors };
    });

    // Update all CSS variables
    for (const { key } of colorVariables) {
      const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
      const color = defaultColors[key as keyof Colors];
      document.documentElement.style.setProperty(cssVariable, color);
    }
  }
</script>

<div class="block">
  <div class="header">
    <h2>Colors</h2>
    <button class="cta-button cancel" on:click={resetColors}>
      Reset Defaults
    </button>
  </div>
  <div class="bar"></div>

  <div class="block inner">
    <div class="color-grid">
      {#each colorVariables as { key, label }}
        <div class="color-item">
          <div class="color-details">
            <span class="color-label">{label}</span>
          </div>
          <div class="color-swatch">
            <div
              class="swatch-box"
              style="background-color: {getCurrentColor(key as keyof Colors)};"
            >
              <input
                type="color"
                value={getCurrentColor(key as keyof Colors)}
                on:input={(e) =>
                  changeColor(
                    key as keyof Colors,
                    (e.target as HTMLInputElement).value
                  )}
                class="color-input"
              />
            </div>
            <div class="hex-value">
              {getCurrentColor(key as keyof Colors)}
            </div>
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  /* CSS remains unchanged */
  .color-swatch {
    width: 50px;
    height: 50px;
    border-radius: 5px;
    cursor: pointer;
    transition: background-color 0.3s ease;
  }

  .color-swatch:hover {
    opacity: 0.8;
  }

  .color-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 24px;
    padding: 24px;
  }

  @media (max-width: 768px) {
    .color-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (max-width: 480px) {
    .color-grid {
      grid-template-columns: 1fr;
    }
  }

  .color-item {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background-color: var(--secondary-bg);
    border-radius: 8px;
    transition:
      transform 0.2s,
      box-shadow 0.2s;
    text-align: center;
  }

  .color-item:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
  }

  .color-details {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: center;
  }

  .color-label {
    font-weight: 500;
    color: var(--text-color);
  }

  .color-swatch {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .swatch-box {
    width: 80px;
    height: 80px;
    border-radius: 8px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
    margin: 0 auto;
    border-width: 1px;
    border-style: solid;
    border-color: var(--inactive-color);
  }

  .color-input {
    opacity: 0;
    position: absolute;
    width: 100%;
    height: 100%;
    cursor: pointer;
  }

  .hex-value {
    font-family: monospace;
    font-size: 12px;
    color: var(--inactive-color);
    text-align: center;
  }

  .block {
    height: calc(100vh - 160px);
  }
</style>
