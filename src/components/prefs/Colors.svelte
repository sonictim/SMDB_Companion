<script lang="ts">
  // Import from main store instead
  import { preferencesStore, updateFontSize } from "../../stores/preferences";
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

  // Function to handle font size changes with decimal precision
  function handleFontSizeChange(event: Event): void {
    const target = event.target as HTMLInputElement;
    // Convert to number with 1 decimal point precision
    const value = parseFloat(parseFloat(target.value).toFixed(1));
    updateFontSize(value);
  }

  // Function to reset font size to default (16px)
  function resetFontSize(): void {
    updateFontSize(16);
  }
</script>

<div class="block">
  <div class="header">
    <h2>Appearance</h2>
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

    <!-- Font Size Slider -->
    <div class="font-size-container">
      <div class="font-size-header">
        <div class="font-size-row">
          <h3 class="font-size-title">Font Size</h3>
          <div class="slider-container">
            <!-- <span class="size-label">12</span> -->
            <div class="slider-wrapper">
              <input
                type="range"
                class="font-size-slider"
                min="12"
                max="20"
                step="0.1"
                value={$preferencesStore.fontSize}
                on:input={handleFontSizeChange}
              />
              <div
                class="slider-marker"
                style="left: calc((16 - 12) / (20 - 12) * 100%);"
                on:click={resetFontSize}
                title="Reset to default (16px)"
              ></div>
              <!-- <div class="slider-value">{$preferencesStore.fontSize}px</div> -->
            </div>
            <!-- <span class="size-label">20</span> -->
          </div>
        </div>
      </div>
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
    gap: 16px;
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
    font-size: var(--font-size-xs);
    color: var(--inactive-color);
    text-align: center;
  }

  .font-size-container {
    margin-top: 0px;
    padding: 12px 24px;
    background-color: var(--secondary-bg);
    border-radius: 8px;
    width: 80vw;
    align-self: center;
  }

  .font-size-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .font-size-row {
    display: flex;
    align-items: center;
    width: 100%;
    justify-content: space-between;
    gap: 16px;
  }

  .font-size-title {
    margin: 0;
    font-size: var(--font-size-lg);
    font-weight: 500;
    color: var(--text-color);
    white-space: nowrap;
    min-width: 100px;
  }

  .slider-container {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 16px;
    justify-content: center;
  }

  .size-label {
    font-family: monospace;
    color: var(--inactive-color);
    font-size: var(--font-size-xs);
    width: 24px;
    text-align: center;
  }

  .slider-wrapper {
    position: relative;
    flex: 1;
    max-width: 400px;
    padding-bottom: 24px;
    display: flex;
    align-items: center;
  }

  .font-size-slider {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: var(--inactive-color);
    -webkit-appearance: none;
    appearance: none;
    outline: none;
    margin: 0;
    position: relative;
    top: 12px;
  }

  .font-size-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent-color);
    cursor: pointer;
    border: 2px solid var(--primary-bg);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    transition: transform 0.1s;
  }

  .font-size-slider::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }

  .font-size-slider::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent-color);
    cursor: pointer;
    border: 2px solid var(--primary-bg);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    transition: transform 0.1s;
  }

  .font-size-slider::-moz-range-thumb:hover {
    transform: scale(1.2);
  }

  .slider-marker {
    position: absolute;
    top: 50%;
    width: 2px;
    height: 14px;
    background-color: var(--text-color);
    transform: translateY(-50%);
    cursor: pointer;
    z-index: 1;
  }

  .slider-marker::after {
    /* content: "16"; */
    position: absolute;
    top: -16px;
    left: -4px;
    font-size: calc(var(--font-size-xs) - 2px);
    color: var(--text-color);
    white-space: nowrap;
  }

  .slider-marker:hover::after {
    content: "Reset to default";
    position: absolute;
    top: -16px;
    left: -40px;
    font-size: calc(var(--font-size-xs) - 2px);
  }

  .slider-value {
    position: absolute;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    font-family: monospace;
    font-size: var(--font-size-xs);
    color: var(--accent-color);
  }

  .header {
    margin-bottom: 0px;
  }

  /* Responsive adjustments for small screens */
  @media (max-width: 580px) {
    .font-size-row {
      flex-direction: column;
      align-items: flex-start;
    }

    .font-size-title {
      margin-bottom: 16px;
    }

    .slider-container {
      width: 100%;
    }
  }
</style>
