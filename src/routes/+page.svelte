<script lang="ts">
  import {
    Database,
    Search as SearchIcon,
    FilesIcon,
    Settings2,
  } from "lucide-svelte";
  import "../styles.css";
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { listen } from "@tauri-apps/api/event";
  import { message } from "@tauri-apps/plugin-dialog";

  // Components
  import SearchComponent from "../components/Search.svelte";
  import SearchSkinnyComponent from "../components/SearchSkinny.svelte";
  import ResultsComponent from "../components/Results.svelte";
  import ResultsSkinnyComponent from "../components/ResultsSkinny.svelte";
  import MetadataComponent from "../components/Metadata.svelte";
  import RegisterComponent from "../components/Register.svelte";
  import RegisterOnlyComponent from "../components/RegisterOnly.svelte";

  // Stores and utilities
  import { preferencesStore } from "../stores/preferences";
  import { databaseStore, openDatabase } from "../stores/database";
  import { checkForUpdates } from "../stores/utils";
  import { checkRegistered } from "../stores/registration";
  import { togglePreferencesWindow, initializeMenu } from "../stores/menu";
  import { applyPreset } from "../stores/presets";
  import type { Preset } from "../stores/types";

  // Component state
  export let activeTab = "search";
  export let isRemove = true;
  export let isRegistered = false;
  let appInitialized = false;
  let initError: unknown = null;
  let presetChangedListener: (() => void) | null = null;

  // Initialize all on mount
  onMount(async () => {
    try {
      // App initialization
      console.log("Starting app initialization");
      await initializeMenu();
      isRegistered = await checkRegistered();
      appInitialized = true;
      console.log("App initialization complete");
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

      // First open dialog and update checks
      if ($preferencesStore.firstOpen) {
        await message(
          "Please be sure to back up your Databases and Audio Files before using this application."
        );
        preferencesStore.update((prefs) => ({ ...prefs, firstOpen: false }));
      }

      // Set up preset listener
      presetChangedListener = await listen("preset-change", (event) => {
        console.log("Preset change event received:", event);

        let presetData = event.payload as { preset: Preset };
        if (presetData?.preset) {
          console.log("Applying preset:", presetData.preset.name);
          applyPreset(presetData.preset);
          preferencesStore.update((prefs) => ({
            ...prefs,
          }));
        } else {
          console.error("Invalid preset data received:", event.payload);
        }
      });

      // Check for updates (non-blocking)
      const updateCheck = await checkForUpdates().catch((err) => {
        console.error("Update check failed:", err);
        return null;
      });

      if (updateCheck?.needsUpdate) {
        const { latest, versionChanges, isBeta } = updateCheck;
        console.log("Update available:", latest);

        const confirmed = await confirm(versionChanges + "\n\nDownload now?", {
          title: "Version " + latest + " is available",
          kind: "info",
        });

        if (confirmed) {
          await openUrl(
            isBeta
              ? "https://smdbc.com/beta.php"
              : "https://smdbc.com/download.php"
          );
        }
      }
    } catch (error) {
      console.error("Fatal error during app initialization:", error);
      initError = error;
    }
  });

  // Clean up on component destruction
  onDestroy(() => {
    if (presetChangedListener) presetChangedListener();
  });
</script>

<svelte:head>
  <script>
    // Force window visibility at the earliest possible moment
    if (window.__TAURI__) {
      window.__TAURI__.window.getCurrent().show();
      console.log("Early window show initiated");
    }
  </script>
</svelte:head>

{#if !appInitialized && !initError}
  <div class="loading-screen">
    <div class="spinner"></div>
    <p>Loading SMDB Companion...</p>
  </div>
{:else if initError}
  <div class="error-screen">
    <h2>Error Starting Application</h2>
    <p>{initError instanceof Error ? initError.message : "Unknown error"}</p>
    <button on:click={() => window.location.reload()}>Retry</button>
  </div>
{/if}

<div class="app-container">
  <!-- Top Bar -->
  <div class="top-bar">
    <div class="top-bar-left">
      <button class="nav-link" on:click={() => openDatabase(false)}>
        <Database size={18} />
        <span style="font-size: 24px;">
          {$databaseStore?.name || "Select Database"}
          {#if $databaseStore}
            <span style="font-size: 14px;"
              >{$databaseStore.size} total records</span
            >
          {/if}
        </span>
      </button>
    </div>
    <div class="top-bar-right">
      <button
        class="nav-link {activeTab === 'search' ? 'active' : ''}"
        on:click={(event) =>
          (activeTab = event.metaKey ? "searchSkinny" : "search")}
        title="Click for Search, ⌘+Click for Split View"
      >
        <div class="flex items-center gap-2">
          <SearchIcon size={18} />
          <span>Search</span>
        </div>
      </button>
      <button
        class="nav-link {['results', 'resultsSkinny'].includes(activeTab)
          ? 'active'
          : ''}"
        on:click={(event) =>
          (activeTab = event.metaKey ? "resultsSkinny" : "results")}
        title="Click for Results, ⌘+Click for Split View"
      >
        <div class="flex items-center gap-2">
          <FilesIcon size={18} />
          <span>Results</span>
        </div>
      </button>
      <button class="nav-link" on:click={togglePreferencesWindow}>
        <div class="flex items-center gap-2">
          <Settings2 size={18} /> Options
        </div>
      </button>
    </div>
  </div>

  <!-- Main Content Area -->
  <main class="content">
    {#if activeTab === "search"}
      <SearchComponent bind:activeTab bind:isRemove />
    {:else if activeTab === "searchSkinny"}
      <div class="grid">
        <SearchSkinnyComponent bind:activeTab bind:isRemove />
        {#if isRegistered}
          <ResultsSkinnyComponent bind:isRemove bind:activeTab />
        {:else}
          <RegisterComponent bind:isRegistered />
        {/if}
      </div>
    {:else if activeTab === "results" || activeTab === "resultsSkinny"}
      {#if isRegistered}
        {#if activeTab === "results"}
          <ResultsComponent bind:isRemove bind:activeTab />
        {:else}
          <ResultsSkinnyComponent bind:isRemove bind:activeTab />
        {/if}
      {:else}
        <RegisterComponent bind:isRegistered />
      {/if}
    {:else if activeTab === "metadata"}
      <MetadataComponent bind:activeTab bind:isRemove />
    {:else if activeTab === "register"}
      <RegisterOnlyComponent bind:isRegistered />
    {/if}
  </main>
</div>

<style>
  .loading-screen,
  .error-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    width: 100vw;
    background-color: var(--primary-bg, #1a1a1a);
    color: var(--color, #ffffff);
  }

  .spinner {
    width: 50px;
    height: 50px;
    border: 5px solid rgba(255, 255, 255, 0.2);
    border-radius: 50%;
    border-top-color: var(--accent-color, #007acc);
    animation: spin 1s ease-in-out infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-screen {
    color: var(--warning-color, #ff4444);
  }

  .error-screen button {
    margin-top: 20px;
    padding: 8px 16px;
    background: var(--accent-color, #007acc);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
</style>
