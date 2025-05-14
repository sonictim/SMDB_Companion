<script lang="ts">
  import {
    ListOrdered,
    ListCheck,
    Tags,
    Palette,
    Keyboard,
  } from "lucide-svelte";
  import "../../styles.css";
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { listen } from "@tauri-apps/api/event";

  import MainComponent from "../../components/prefs/Main.svelte";
  import MatchComponent from "../../components/prefs/Match.svelte";
  import OrderComponent from "../../components/prefs/Order.svelte";
  import TagsComponent from "../../components/prefs/Tags.svelte";
  import SelectComponent from "../../components/prefs/Select.svelte";
  import ColorsComponent from "../../components/prefs/Colors.svelte";
  import HotkeysComponent from "../../components/prefs/Hotkeys.svelte";
  import { preferencesStore } from "../../stores/preferences";
  import { hotkeysStore } from "../../stores/hotkeys";
  import {
    presetsStore,
    loadPreset,
    savePreset,
    deletePreset,
    applyPreset,
  } from "../../stores/presets";
  import type { Preset } from "../../stores/types";

  let presetChangedListener: (() => void) | null = null;
  let preferencesChangedListener: (() => void) | null = null;
  let newPreset: string = "";
  let selectedPreset: string = "";
  export let activeTab = "matchCriteria";

  // Tab configuration for easy maintenance
  const tabs = [
    { id: "matchCriteria", label: "Match Criteria", icon: ListCheck },
    { id: "preservationOrder", label: "Preservation Order", icon: ListOrdered },
    { id: "audiosuiteTags", label: "Tags Manager", icon: Tags },
    { id: "hotkeys", label: "Keyboard Shortcuts", icon: Keyboard },
    { id: "colors", label: "Colors", icon: Palette },
  ];

  onMount(async () => {
    // Listen for preset changes
    try {
      const currentPrefs = get(preferencesStore);

      // Update CSS variables from current preferences
      if (currentPrefs?.colors) {
        Object.entries(currentPrefs.colors).forEach(([key, value]) => {
          const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
          document.documentElement.style.setProperty(
            cssVariable,
            String(value)
          );
        });
      }

      presetChangedListener = await listen(
        "preset-change",
        (event: { payload: { preset: Preset } }) => {
          console.log("Preset change event received:", event);
          let presetData = event.payload as { preset: Preset };

          if (presetData && presetData.preset) {
            console.log("Applying preset:", presetData.preset.name);
            applyPreset(presetData.preset);
          } else {
            console.error("Invalid preset data received:", event.payload);
          }
        }
      );
      preferencesChangedListener = await listen(
        "preference-change",
        async () => {
          console.log("Preference change detected, reloading preferences");

          // Load the latest preferences from localStorage
          // Fix: Use 'preferencesInfo' to match the store's initialization key
          const storedPrefs = localStorage.getItem("preferencesInfo");
          if (storedPrefs) {
            try {
              const latestPrefs = JSON.parse(storedPrefs);
              preferencesStore.set(latestPrefs);
            } catch (error) {
              console.error("Error parsing stored preferences:", error);
            }
          }
        }
      );

      // Listen for hotkey changes to sync between windows
      await listen("hotkey-change", async () => {
        console.log("Hotkey change detected in preferences window");
        // Reload hotkeys from localStorage
        const storedHotkeys = localStorage.getItem("hotkeys");
        if (storedHotkeys) {
          try {
            const latestHotkeys = JSON.parse(storedHotkeys);
            hotkeysStore.set(latestHotkeys);
          } catch (error) {
            console.error("Error parsing hotkeys:", error);
          }
        }
      });
    } catch (error) {
      console.error("Error setting up listeners:", error);
    }
  });

  onDestroy(() => {
    if (preferencesChangedListener) preferencesChangedListener();
    if (presetChangedListener) presetChangedListener();
  });

  function handleSavePreset() {
    if (!newPreset) return;
    savePreset(newPreset);
    selectedPreset = newPreset;
    console.log("Preset saved:", newPreset);
    newPreset = "";
  }
</script>

<div class="app-container">
  <div class="top-bar">
    <div class="top-bar-left">
      {#each tabs as tab}
        <button
          class="nav-link {activeTab === tab.id ? 'active' : ''}"
          on:click={() => (activeTab = tab.id)}
        >
          <div class="flex items-center gap-2">
            <svelte:component this={tab.icon} size={18} />
            <span>{tab.label}</span>
          </div>
        </button>
      {/each}
    </div>
  </div>

  <main class="content">
    <div>
      {#if activeTab === "mainPref"}
        <MainComponent />
      {:else if activeTab === "matchCriteria"}
        <MatchComponent />
      {:else if activeTab === "preservationOrder"}
        <OrderComponent />
      {:else if activeTab === "audiosuiteTags"}
        <TagsComponent />
      {:else if activeTab === "autoSelect"}
        <SelectComponent />
      {:else if activeTab === "colors"}
        <ColorsComponent />
      {:else if activeTab === "hotkeys"}
        <HotkeysComponent />
      {/if}
    </div>

    <div class="preset-bar">
      <button
        class="cta-button small"
        on:click={handleSavePreset}
        disabled={!newPreset}
      >
        Save Preset
      </button>
      <input
        type="text"
        class="input-field"
        placeholder="Enter New Configuration Preset Name"
        bind:value={newPreset}
      />

      <select
        class="select-field"
        bind:value={selectedPreset}
        on:change={() => loadPreset(selectedPreset)}
      >
        <option value="">Select a preset...</option>
        {#each $presetsStore as p}
          <option value={p.name}>{p.name}</option>
        {/each}
      </select>
      <button
        class="cta-button small cancel"
        on:click={() => deletePreset(selectedPreset)}
        disabled={!selectedPreset}
      >
        Delete
      </button>
    </div>
  </main>
</div>

<style>
  .top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: var(--topbar-color);
    padding: 10px;
    width: 100%;
  }

  .top-bar-left {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 30px;
  }

  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .content {
    flex-grow: 1;
    overflow-y: auto;
  }

  .preset-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 10px;
    background-color: var(--secondary-bg);
    color: var(--text-color);
    width: calc(100% + 40px);
    margin-top: 16px;
    margin-left: -20px;
    margin-right: 20px;
    position: sticky;
    bottom: 0;
  }

  .preset-bar .cta-button:first-child {
    margin-left: 30px;
  }

  .preset-bar .input-field,
  .preset-bar .select-field {
    margin-right: 10px;
  }

  .preset-bar .cta-button.cancel {
    margin-right: 25px;
  }
</style>
