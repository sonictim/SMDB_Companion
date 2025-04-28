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
  } from "../../stores_OLD/store";
  import type { Preferences } from "../../stores_OLD/store";
  import { get } from "svelte/store";
  import { listen } from "@tauri-apps/api/event";
  import { initColorHandling } from "../../stores_OLD/store";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  onMount(() => {
    initColorHandling();
  });

  let newPreset: string;
  let selectedPreset: string = ""; // Bind this to <select>
  export let activeTab = "matchCriteria"; // Ensure this matches below
  $: console.log("Active Tab:", activeTab);
  $: pref = $preferencesStore;
  $: presets = $PresetsStore;

  // $: if ($preferencesStore && $preferencesStore.algorithms) {
  //     algorithmsStore.set($preferencesStore.algorithms);
  // }

  function closeWindow() {
    let newPreset = "closewindow";
    let window = getCurrentWebviewWindow();
    window.emit("close-window");
    window.close();
    window.onCloseRequested(() => {
      window.emit("close-window");
    });
  }

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
    closeWindow();
  }
</script>

<div class="grid-container">
  <div class="block">
    <h3 style="margin-bottom: 5px">Save Preset:</h3>

    <input
      type="text"
      class="input-field"
      placeholder="Enter Preset Name"
      style=" margin: 10px"
      bind:value={newPreset}
    />
    <span>
      <button class="cta-button small" on:click={savePreset}>Save</button>
      <button class="cta-button small cancel" on:click={() => closeWindow()}
        >Cancel</button
      ></span
    >
  </div>
</div>

<style>
  .cta-button {
    margin: 10px;
  }
  .grid-container {
    display: grid;
    /* height: calc(100vh - 10px); */
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    padding: 20px;
  }
  .block {
    background-color: var(--secondary-bg);
    border-radius: 10px;
    padding: 20px;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    height: 85vh;
    width: 87vw;
    margin-top: -8px;
  }
</style>
