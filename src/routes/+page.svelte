<script lang="ts">
  import "../styles.css";
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { confirm, message } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { listen } from "@tauri-apps/api/event";

  // Components
  import Header from "../components/Header.svelte";
  import SearchComponent from "../components/Search.svelte";
  import SearchSkinnyComponent from "../components/SearchSkinny.svelte";
  import ResultsComponent from "../components/Results.svelte";
  import ResultsSkinnyComponent from "../components/ResultsSkinny.svelte";
  import MetadataComponent from "../components/Metadata.svelte";
  import RegisterComponent from "../components/Register.svelte";
  import RegisterOnlyComponent from "../components/RegisterOnly.svelte";
  import AdvancedComponent from "../components/AdvancedMode.svelte";

  // Stores and utilities
  import {
    preferencesStore,
    updateAlgorithmOrder,
    addMissingPrefs,
  } from "../stores/preferences";
  import { databaseStore } from "../stores/database";
  import { checkForUpdates } from "../stores/utils";
  import { checkRegistered } from "../stores/registration";
  import { initializeMenu, viewStore, showSearchView } from "../stores/menu";
  import { applyPreset } from "../stores/presets";
  import type { Preset } from "../stores/types";

  // Component state
  export let isRemove = true;
  export let isRegistered = false;
  let appInitialized = false;
  let initError: unknown = null;
  let presetChangedListener: (() => void) | null = null;
  let preferencesChangedListener: (() => void) | null = null;

  // Use the viewStore instead of local variables
  $: view = $viewStore;

  // Initialize all on mount
  onMount(async () => {
    try {
      // App initialization
      console.log("Starting app initialization");
      addMissingPrefs();
      await initializeMenu();
      isRegistered = await checkRegistered();
      appInitialized = true;
      console.log("App initialization complete");
      updateAlgorithmOrder();
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
        preferencesStore.update((prefs: any) => ({
          ...prefs,
          firstOpen: false,
        }));
      }

      // Set up preset listener
      presetChangedListener = await listen(
        "preset-change",
        (event: { payload: { preset: Preset } }) => {
          console.log("Preset change event received:", event);

          let presetData = event.payload as { preset: Preset };
          if (presetData?.preset) {
            console.log("Applying preset:", presetData.preset.name);
            applyPreset(presetData.preset);
            preferencesStore.update((prefs: any) => ({
              ...prefs,
            }));
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

              // Update any UI elements that depend on preferences
              // e.g., CSS variables
            } catch (error) {
              console.error("Error parsing stored preferences:", error);
            }
          }
        }
      );

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
    if (view === "results") showSearchView();
    if (presetChangedListener) presetChangedListener();
    if (preferencesChangedListener) preferencesChangedListener();
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

{#if view === "nofrills"}
  <AdvancedComponent />
{:else}
  <div class="app-container">
    <!-- Top Bar -->
    <Header />

    <!-- Main Content Area -->
    <main class="content">
      {#if view === "metadata"}
        <MetadataComponent bind:isRemove />
      {:else if view === "registration"}
        <RegisterOnlyComponent bind:isRegistered />
      {:else if view === "split"}
        <div class="grid">
          <SearchSkinnyComponent bind:isRemove />
          {#if isRegistered}
            <ResultsSkinnyComponent bind:isRemove />
          {:else}
            <RegisterComponent bind:isRegistered />
          {/if}
        </div>
      {:else if view === "search"}
        <SearchComponent bind:isRemove />
      {:else if view === "results"}
        {#if isRegistered}
          <ResultsComponent
            {isRemove}
            selectedDb={$databaseStore?.path || null}
          />
        {:else}
          <RegisterComponent bind:isRegistered />
        {/if}
      {/if}
    </main>
  </div>
{/if}

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
