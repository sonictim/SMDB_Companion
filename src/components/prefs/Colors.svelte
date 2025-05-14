<script lang="ts">
  // Import from main store instead
  import { preferencesStore } from "../../stores/preferences";
  import type { Colors } from "../../stores/types";
  import {
    colorVariables,
    changeColor,
    resetColors,
  } from "../../stores/colors";

  // Use the $: syntax to ensure preferences stays reactive
  $: preferences = $preferencesStore;

  // Function to get current color value - this is now reactive
  function getCurrentColor(colorKey: string): string {
    return preferences?.colors[colorKey as keyof Colors] || "";
  }

  // Wrapper function to handle the type conversion from string to keyof Colors
  function handleColorChange(key: string, value: string): void {
    changeColor(key as keyof Colors, value);
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
              style="background-color: {getCurrentColor(key)};"
            >
              <input
                type="color"
                value={getCurrentColor(key)}
                on:input={(e) => handleColorChange(key, e.currentTarget.value)}
                class="color-input"
              />
            </div>
            <div class="hex-value">
              {getCurrentColor(key)}
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
