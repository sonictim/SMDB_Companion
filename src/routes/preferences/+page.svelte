<script lang="ts">
  import {
    ListOrdered,
    ListCheck,
    ListChecks,
    Tags,
    Palette,
    Settings2,
  } from "lucide-svelte";
  import "../../styles.css";
  import { onMount } from "svelte";

  import MainComponent from "../../components/prefs/Main.svelte";
  import MatchComponent from "../../components/prefs/Match.svelte";
  import OrderComponent from "../../components/prefs/Order.svelte";
  import TagsComponent from "../../components/prefs/Tags.svelte";
  import SelectComponent from "../../components/prefs/Select.svelte";
  import ColorsComponent from "../../components/prefs/Colors.svelte";
  import {
    preferencesStore,
    PresetsStore,
    defaultPreferences,
    defaultAlgorithms,
    // algorithmsStore,
  } from "../../store";
  import type {
    Algorithm,
    Colors,
    Preferences,
    PreservationLogic,
  } from "../../store";
  import { get } from "svelte/store";
  import { listen } from "@tauri-apps/api/event";
  import { initColorHandling } from "../../store";

  onMount(() => {
    initColorHandling();
  });

  // Use the store directly instead of assigning to `pref`

  let newPreset: string;
  let selectedPreset: string = ""; // Bind this to <select>
  export let activeTab = "matchCriteria"; // Ensure this matches below
  $: console.log("Active Tab:", activeTab);
  $: pref = $preferencesStore;
  $: presets = $PresetsStore;

  // $: if ($preferencesStore && $preferencesStore.algorithms) {
  //     algorithmsStore.set($preferencesStore.algorithms);
  // }

  function savePreset() {
    const trimmedPreset = newPreset?.trim();

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
        PresetsStore.update((presets) => {
          presets[existingPresetIndex].pref = get(preferencesStore); // Update the preferences
          return [...presets]; // Return updated presets
        });
        console.log("Preset updated:", trimmedPreset);
      } else {
        // If it doesn't exist, create a new preset
        PresetsStore.update((presets) => [
          ...presets,
          { name: trimmedPreset, pref: get(preferencesStore) },
        ]);
        console.log("Preset saved:", trimmedPreset);
      }

      selectedPreset = trimmedPreset; // Update the selected preset
      newPreset = ""; // Clear input after saving
    }
  }

  function loadPreset() {
    // First create a fresh copy of default preferences as our base
    const defaultPrefs = structuredClone(defaultPreferences);

    if (selectedPreset === "Default") {
      // For Default, simply use defaultPreferences directly
      preferencesStore.set(defaultPrefs);

      // Apply colors
      applyColors(defaultPrefs.colors);

      console.log("Default preferences restored");
      return;
    }

    // For other presets, find the preset object
    const presetObj = presets.find((p) => p.name === selectedPreset);

    if (presetObj) {
      // Create a deep copy of the preset's preferences
      const prefCopy = structuredClone(presetObj.pref);

      // Recursively merge with defaults to ensure all properties exist
      const mergedPrefs = deepMerge(defaultPrefs, prefCopy);

      // Special handling for arrays that should be replaced, not merged
      if (Array.isArray(prefCopy.algorithms)) {
        mergedPrefs.algorithms = prefCopy.algorithms;
      }

      if (Array.isArray(prefCopy.match_criteria)) {
        mergedPrefs.match_criteria = prefCopy.match_criteria;
      }

      if (Array.isArray(prefCopy.tags)) {
        mergedPrefs.tags = prefCopy.tags;
      }

      if (Array.isArray(prefCopy.autoselects)) {
        mergedPrefs.autoselects = prefCopy.autoselects;
      }

      if (Array.isArray(prefCopy.preservation_order)) {
        mergedPrefs.preservation_order = prefCopy.preservation_order;
      }

      // Ensure algorithms has the correct structure
      if (!mergedPrefs.algorithms || !Array.isArray(mergedPrefs.algorithms)) {
        console.warn("Invalid algorithms in preset, using defaults");
        mergedPrefs.algorithms = defaultAlgorithms;
      }

      // Ensure all required properties have valid values
      // firstOpen is required to be a boolean, not undefined
      if (mergedPrefs.firstOpen === undefined) {
        mergedPrefs.firstOpen = false;
      }

      // Update the store with properly merged preferences
      preferencesStore.set(mergedPrefs as Preferences);

      // Apply colors
      applyColors(mergedPrefs.colors);

      console.log("Loaded preset:", selectedPreset);
    }
  }

  // Helper function to deeply merge objects, preferring source values
  function deepMerge(
    target: {
      [x: string]: any;
      firstOpen?: boolean;
      match_criteria?: string[];
      ignore_filetype?: boolean;
      autoselects?: string[];
      tags?: string[];
      preservation_order?: PreservationLogic[];
      columns?: string[];
      display_all_records?: boolean;
      safety_db?: boolean;
      safety_db_tag?: string;
      erase_files?: string;
      strip_dual_mono?: boolean;
      waveform_search_type?: string;
      similarity_threshold?: number;
      store_waveforms?: boolean;
      fetch_waveforms?: boolean;
      colors?: Colors;
      algorithms?: Algorithm[];
    },
    source: {
      [x: string]: any;
      firstOpen?: boolean;
      match_criteria?: string[];
      ignore_filetype?: boolean;
      autoselects?: string[];
      tags?: string[];
      preservation_order?: PreservationLogic[];
      columns?: string[];
      display_all_records?: boolean;
      safety_db?: boolean;
      safety_db_tag?: string;
      erase_files?: string;
      strip_dual_mono?: boolean;
      waveform_search_type?: string;
      similarity_threshold?: number;
      store_waveforms?: boolean;
      fetch_waveforms?: boolean;
      colors?: Colors;
      algorithms?: Algorithm[];
    }
  ) {
    const output = { ...target };

    if (isObject(target) && isObject(source)) {
      Object.keys(source).forEach((key) => {
        if (isObject(source[key])) {
          if (!(key in target)) {
            Object.assign(output, { [key]: source[key] });
          } else {
            output[key] = deepMerge(target[key], source[key]);
          }
        } else {
          Object.assign(output, { [key]: source[key] });
        }
      });
    }

    return output;
  }

  // Helper to check if something is an object
  function isObject(item: any) {
    return item && typeof item === "object" && !Array.isArray(item);
  }

  // Helper function to apply colors to the document
  function applyColors(colors?: { [s: string]: unknown } | ArrayLike<unknown>) {
    if (!colors) return;

    Object.entries(colors).forEach(([key, value]) => {
      const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
      document.documentElement.style.setProperty(cssVariable, String(value));
    });
  }
  function deletePreset() {
    if (!selectedPreset || selectedPreset === "Default") {
      console.log("Cannot delete the Default preset.");
      return;
    }

    // Remove the selected preset
    PresetsStore.update((presets) =>
      presets.filter((p) => p.name !== selectedPreset)
    );

    console.log("Preset deleted:", selectedPreset);

    // Update the selection to another preset or default
    selectedPreset = "";
  }

  onMount(() => {
    // Get current preferences when component mounts
    const currentPrefs = get(preferencesStore);

    // Update CSS variables from current preferences
    if (currentPrefs?.colors) {
      Object.entries(currentPrefs.colors).forEach(([key, value]) => {
        const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
        document.documentElement.style.setProperty(cssVariable, String(value));
      });
    }

    console.log("Preferences window mounted, colors updated");
  });
</script>

<div class="app-container">
  <div class="top-bar">
    <div class="top-bar-left">
      <!-- <button
                class="nav-link {activeTab === 'mainPref' ? 'active' : ''}"
                on:click={() => (activeTab = "mainPref")}
            >
                <div class="flex items-center gap-2">
                    <Settings2 size={18} />
                    <span>Options</span>
                </div>
            </button> -->
      <button
        class="nav-link {activeTab === 'matchCriteria' ? 'active' : ''}"
        on:click={() => (activeTab = "matchCriteria")}
      >
        <div class="flex items-center gap-2">
          <ListCheck size={18} />
          <span>Match Criteria</span>
        </div>
      </button>
      <button
        class="nav-link {activeTab === 'preservationOrder' ? 'active' : ''}"
        on:click={() => (activeTab = "preservationOrder")}
      >
        <div class="flex items-center gap-2">
          <ListOrdered size={18} />
          <span>Preservation Order</span>
        </div>
      </button>
      <button
        class="nav-link {activeTab === 'Tags Editor' ? 'active' : ''}"
        on:click={() => (activeTab = "audiosuiteTags")}
      >
        <div class="flex items-center gap-2">
          <Tags size={18} />
          <span>Tags Manager</span>
        </div>
      </button>
      <!-- <button 
                class="nav-link {activeTab === 'autoSelect' ? 'active' : ''}"
                on:click={() => activeTab = 'autoSelect'}
            >
                <div class="flex items-center gap-2">
                    <ListChecks size={18} />
                    <span>AutoSelect Strings</span>
                </div>
            </button> -->
      <button
        class="nav-link {activeTab === 'colors' ? 'active' : ''}"
        on:click={() => (activeTab = "colors")}
      >
        <div class="flex items-center gap-2">
          <Palette size={18} />
          <span>Colors</span>
        </div>
      </button>
    </div>
  </div>

  <main class="content" style="margin-bottom: 0px">
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
      {/if}
    </div>
    <div
      class="bar"
      style="width: calc(100% + 40px); margin-top: 16px; margin-left: -20px; margin-right: 20px;"
    >
      <button
        class="cta-button small"
        style="margin-left: 30px"
        on:click={savePreset}>Save:</button
      >
      <input
        type="text"
        class="input-field"
        placeholder="Enter New Configuration Preset Name"
        style=" margin-right: 10px;"
        bind:value={newPreset}
      />
      <!-- <button class="cta-button small" on:click={loadPreset}>
                Load:
            </button> -->
      <select
        class="select-field"
        style="margin-right: 10px"
        bind:value={selectedPreset}
        on:change={loadPreset}
      >
        {#each presets as p}
          <option value={p.name}>{p.name}</option>
        {/each}
      </select>
      <button
        class="cta-button small cancel"
        style="margin-right: 25px;"
        on:click={deletePreset}
      >
        Delete
      </button>
    </div>
  </main>
</div>

<style>
  .bar {
    background-color: var(--primary-bg);
  }

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
    /* margin-bottom: 40px; */
  }

  .content {
    flex-grow: 1;
    overflow-y: auto;
  }

  .bar {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 10px;
    background-color: var(--secondary-bg);
    color: var(--text-color);
    width: 100%;
    position: sticky;
    bottom: 0;
  }
</style>
