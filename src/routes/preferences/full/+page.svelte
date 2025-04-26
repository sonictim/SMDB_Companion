<script lang="ts">
  import {
    ListOrdered,
    ListCheck,
    Tags,
    Palette,
    Settings2,
  } from "lucide-svelte";
  import "../../../styles.css";
  import { onMount } from "svelte";
  import { get } from "svelte/store";

  // Import all your components
  import MatchComponent from "../../../components/prefs/Match.svelte";
  import OrderComponent from "../../../components/prefs/Order.svelte";
  import TagsComponent from "../../../components/prefs/Tags.svelte";
  import ColorsComponent from "../../../components/prefs/Colors.svelte";

  import { preferencesStore } from "../../../stores/preferences";
  import {
    presetsStore,
    loadPreset,
    savePreset,
    deletePreset,
  } from "../../../stores/presets";
  import { applyColors, initColorHandling } from "../../../stores/colors";

  // Add debug mode detection
  const IS_DEV_MODE = import.meta.env.DEV || false;

  // Component state
  let activeTab = "matchCriteria";
  let loadingStage = "initializing";
  let error: unknown = null;
  let componentError: null = null;
  let newPreset = "";
  let selectedPreset = "";

  // Debug info visibility - only enabled in dev mode
  let showDebugInfo = false;

  function toggleDebugInfo() {
    showDebugInfo = !showDebugInfo;
  }

  // Reactive variables
  $: presets = $presetsStore || [];
  $: pref = $preferencesStore;

  onMount(async () => {
    try {
      loadingStage = "applying-colors";
      console.log("Preferences mount start");

      // Initialize color handling
      initColorHandling();

      // Get current preferences and apply colors
      const currentPrefs = get(preferencesStore);
      if (currentPrefs?.colors) {
        applyColors(currentPrefs.colors);
      }

      loadingStage = "ready";
      console.log("Preferences mount complete");
    } catch (e) {
      error = e;
      loadingStage = "error";
      console.error("Error in onMount:", e);
    }
  });

  function handleComponentError(event: { detail: any }) {
    componentError = event.detail;
  }
</script>

<svelte:head>
  <title>Preferences</title>
</svelte:head>

<div class="app-container">
  <!-- Debug tools only available in development mode -->
  {#if IS_DEV_MODE}
    {#if showDebugInfo}
      <div class="debug-info">
        <button class="close-button" on:click={toggleDebugInfo}>×</button>
        <p>State: {loadingStage}</p>
        <p>Active Tab: {activeTab}</p>
      </div>
    {:else}
      <button class="show-debug-button" on:click={toggleDebugInfo}>Debug</button
      >
    {/if}
  {/if}

  {#if error}
    <div class="error-banner">
      <h3>Error loading page</h3>
      <pre>{String(error)}</pre>
      <button on:click={() => window.location.reload()}>Reload</button>
    </div>
  {:else}
    <!-- Updated top navigation bar styling to match main app -->
    <div class="top-bar">
      <div class="top-bar-left">
        <button
          class="nav-link {activeTab === 'matchCriteria' ? 'active' : ''}"
          on:click={() => (activeTab = "matchCriteria")}
        >
          <div class="flex items-center gap-2">
            <ListCheck size={20} />
            <span>Match Criteria</span>
          </div>
        </button>
        <button
          class="nav-link {activeTab === 'preservationOrder' ? 'active' : ''}"
          on:click={() => (activeTab = "preservationOrder")}
        >
          <div class="flex items-center gap-2">
            <ListOrdered size={20} />
            <span>Preservation Order</span>
          </div>
        </button>
        <button
          class="nav-link {activeTab === 'audiosuiteTags' ? 'active' : ''}"
          on:click={() => (activeTab = "audiosuiteTags")}
        >
          <div class="flex items-center gap-2">
            <Tags size={20} />
            <span>Tags Manager</span>
          </div>
        </button>
        <button
          class="nav-link {activeTab === 'colors' ? 'active' : ''}"
          on:click={() => (activeTab = "colors")}
        >
          <div class="flex items-center gap-2">
            <Palette size={20} />
            <span>Colors</span>
          </div>
        </button>
      </div>
    </div>

    <!-- Component error display -->
    {#if componentError}
      <div class="component-error">
        <h3>Component Error</h3>
        <pre>{String(componentError)}</pre>
        <button on:click={() => (componentError = null)}>Dismiss</button>
      </div>
    {/if}

    <!-- Main content area -->
    <main class="content" style="margin-bottom: 0px">
      <div>
        {#if activeTab === "matchCriteria"}
          <MatchComponent on:error={handleComponentError} />
        {:else if activeTab === "preservationOrder"}
          <OrderComponent on:error={handleComponentError} />
        {:else if activeTab === "audiosuiteTags"}
          <TagsComponent on:error={handleComponentError} />
        {:else if activeTab === "colors"}
          <ColorsComponent on:error={handleComponentError} />
        {/if}
      </div>

      <!-- Bottom preset control bar -->
      <div
        class="bar"
        style="width: calc(100% + 40px); margin-top: 16px; margin-left: -20px; margin-right: 20px;"
      >
        <button
          class="cta-button small"
          style="margin-left: 30px"
          on:click={() => savePreset(newPreset)}
        >
          Save:
        </button>
        <input
          type="text"
          class="input-field"
          placeholder="Enter New Configuration Preset Name"
          style="margin-right: 10px;"
          bind:value={newPreset}
        />
        <select
          class="select-field"
          style="margin-right: 10px"
          bind:value={selectedPreset}
          on:change={(e) => loadPreset((e.target as HTMLSelectElement).value)}
        >
          <option value="">Select Preset</option>
          {#each presets as p}
            <option value={p.name}>{p.name}</option>
          {/each}
        </select>
        <button
          class="cta-button small cancel"
          style="margin-right: 25px;"
          on:click={() => deletePreset(selectedPreset)}
          disabled={!selectedPreset}
        >
          Delete
        </button>
      </div>
    </main>
  {/if}
</div>

<style>
  .debug-info {
    position: fixed;
    bottom: 10px;
    left: 10px;
    background: rgba(0, 0, 0, 0.8);
    color: white;
    padding: 8px 12px;
    font-size: 11px;
    z-index: 9999;
    border-radius: 4px;
    max-width: 200px;
  }

  .close-button {
    position: absolute;
    top: 2px;
    right: 2px;
    background: transparent;
    border: none;
    color: white;
    font-size: 16px;
    cursor: pointer;
    padding: 0 5px;
  }

  .show-debug-button {
    position: fixed;
    bottom: 10px;
    left: 10px;
    background: rgba(0, 0, 0, 0.5);
    color: white;
    border: none;
    padding: 5px 8px;
    font-size: 11px;
    border-radius: 4px;
    cursor: pointer;
    z-index: 9999;
  }

  .error-banner {
    background-color: #ff5555;
    color: white;
    padding: 20px;
    margin: 20px;
    border-radius: 5px;
    font-family: monospace;
    white-space: pre-wrap;
  }

  .component-error {
    background-color: rgba(255, 85, 85, 0.8);
    color: white;
    padding: 15px;
    margin: 10px 20px;
    border-radius: 5px;
    font-family: monospace;
  }

  .bar {
    background-color: var(--primary-bg);
  }

  /* Updated top bar styling to match main app */
  .top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: var(--topbar-color);
    padding: 0;
    width: 100%;
    height: 60px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .top-bar-left {
    display: flex;
    justify-content: space-around;
    align-items: center;
    width: 100%;
    height: 100%;
    padding: 0 20px;
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

  .flex {
    display: flex;
  }

  .items-center {
    align-items: center;
  }

  .gap-2 {
    gap: 10px;
  }

  .nav-link {
    height: 100%;
    padding: 0 20px;
    display: flex;
    align-items: center;
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--primary-bg);
    font-weight: 500;
    font-size: 16px;
    transition: background-color 0.2s;
  }

  .nav-link:hover {
    background-color: rgba(0, 0, 0, 0.1);
  }

  .nav-link.active {
    background-color: rgba(0, 0, 0, 0.15);
    font-weight: 600;
  }
</style>
