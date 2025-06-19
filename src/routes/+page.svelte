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
  import ResultsComponent from "../components/Results.svelte";
  import MetadataComponent from "../components/Metadata.svelte";
  import RegistrationComponent from "../components/Registration.svelte";
  import SplitComponent from "../components/Split.svelte";
  import NoFrillsComponent from "../components/NoFrills.svelte";
  import SearchFolderPopup from "../components/popups/SearchFolders.svelte";
  import ServerConnectPopup from "../components/popups/ServerConnect.svelte";
  import SearchPopup from "../components/popups/Search.svelte";
  import MetadataPopup from "../components/popups/Metadata.svelte";
  import RemovePopup from "../components/popups/Remove.svelte";

  // Stores and utilities
  import {
    preferencesStore,
    updateAlgorithmOrder,
    addMissingPrefs,
  } from "../stores/preferences";
  import { databaseStore } from "../stores/database";
  import { checkForUpdates } from "../stores/utils";
  import { checkRegistered, isRegistered } from "../stores/registration";
  import {
    initializeMenu,
    viewStore,
    showPopup,
    Popup,
    clearPopups,
    RemovePopup as RP,
  } from "../stores/menu";
  import { applyPreset } from "../stores/presets";
  import { hotkeysStore } from "../stores/hotkeys";
  import type { Preset } from "../stores/types";
  import { cancelSearch } from "../stores/status";
  import Remove from "../components/popups/Remove.svelte";

  // Component state
  let appInitialized = false;
  let initError: unknown = null;
  let presetChangedListener: (() => void) | null = null;
  let preferencesChangedListener: (() => void) | null = null;
  let hotkeyChangeListener: (() => void) | null = null;

  // Use the viewStore instead of local variables
  $: view = $viewStore;

  // Initialize all on mount
  onMount(async () => {
    try {
      window.addEventListener("keydown", handleGlobalKeydown);
      // App initialization
      console.log("Starting app initialization");
      addMissingPrefs();

      // Ensure hotkeys defaults are applied - critical for fresh installations
      const { checkForNewDefaults } = await import("../stores/hotkeys");
      checkForNewDefaults();

      await initializeMenu();
      const reg = await checkRegistered();
      isRegistered.set(reg);

      // Apply font size from preferences
      const { applyFontSize } = await import("../stores/colors");
      if ($preferencesStore.fontSize) {
        await applyFontSize($preferencesStore.fontSize);
      }

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

      // Listen for preset changes
      presetChangedListener = await listen(
        "preset-change",
        (event: { payload: { preset: Preset } }) => {
          console.log("Preset change event received:", event);
          let presetData = event.payload as { preset: Preset };
          if (presetData && presetData.preset) {
            console.log("Applying preset:", presetData.preset.name);
            applyPreset(presetData.preset);
          }
        }
      );

      // Listen for font size changes
      listen("font-size-updated", (event) => {
        const { fontSize } = event.payload as { fontSize: number };
        if (fontSize) {
          // Update main font size
          document.documentElement.style.setProperty(
            "--font-size",
            `${fontSize}px`
          );

          // Update derived font size variables
          document.documentElement.style.setProperty(
            "--font-size-xs",
            `${fontSize - 4}px`
          );
          document.documentElement.style.setProperty(
            "--font-size-sm",
            `${fontSize - 3}px`
          );
          document.documentElement.style.setProperty(
            "--font-size-md",
            `${fontSize - 2}px`
          );
          document.documentElement.style.setProperty(
            "--font-size-lg",
            `${fontSize + 2}px`
          );
          document.documentElement.style.setProperty(
            "--font-size-xl",
            `${fontSize + 8}px`
          );
        }
      }); // Listen for color changes from preferences window
      await listen("color-updated", (event) => {
        const { colorKey, cssVariable, newColor } = event.payload as {
          colorKey: string;
          cssVariable: string;
          newColor: string;
        };
        if (cssVariable && newColor) {
          // Update the CSS variable in the main window
          document.documentElement.style.setProperty(cssVariable, newColor);
          console.log(`[MAIN] Updated ${cssVariable} to ${newColor}`);
        }
      });

      // Listen for color scheme changes
      await listen("color-scheme-loaded", async (event) => {
        const { name, colors } = event.payload as {
          name: string;
          colors: any;
        };
        console.log(`[MAIN] Color scheme loaded: ${name}`);

        // Apply the colors to the main window
        if (colors) {
          Object.entries(colors).forEach(([key, value]) => {
            const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
            document.documentElement.style.setProperty(
              cssVariable,
              String(value)
            );
          });
        }
      });

      await listen("color-scheme-saved", (event) => {
        const { name } = event.payload as { name: string };
        console.log(`[MAIN] Color scheme saved: ${name}`);
      });

      await listen("color-scheme-deleted", (event) => {
        const { name } = event.payload as { name: string };
        console.log(`[MAIN] Color scheme deleted: ${name}`);
      });

      await listen("color-schemes-reset", () => {
        console.log(`[MAIN] Color schemes reset to defaults`);
      });

      // Listen for preference changes to sync between windows
      preferencesChangedListener = await listen(
        "preference-change",
        async () => {
          console.log(
            "🔔 [MAIN] Preference change event received in main window"
          );
          // Load the latest preferences from localStorage
          const storedPrefs = localStorage.getItem("preferencesInfo");
          if (storedPrefs) {
            try {
              const latestPrefs = JSON.parse(storedPrefs);
              console.log(
                "🔄 [MAIN] Updating preferences store with new batch_size:",
                latestPrefs.batch_size
              );
              preferencesStore.set(latestPrefs);
              console.log(
                "✅ [MAIN] Main window preferences updated successfully"
              );
            } catch (error) {
              console.error("Error parsing stored preferences:", error);
            }
          }
        }
      );

      // Listen for hotkey changes to sync between windows
      hotkeyChangeListener = await listen("hotkey-change", async () => {
        console.log("Hotkey change detected in main window");
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

      // Check for updates in the background
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
    if (view === "results") view = "search";
    window.removeEventListener("keydown", handleGlobalKeydown);
    if (presetChangedListener) presetChangedListener();
    if (preferencesChangedListener) preferencesChangedListener();
    if (hotkeyChangeListener) hotkeyChangeListener();
  });

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      clearPopups();

      // Add other popup stores here
      // if ($otherPopupStore) {
      //   otherPopupStore.set(false);
      // }

      // For Tauri windows, you could also close the window
      // getCurrentWindow().close();
    }
    if (event.key === "~") {
      RP();
    }
  }
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
  <NoFrillsComponent />
{:else}
  <div class="app-container">
    <!-- Top Bar -->
    <Header />

    <!-- Main Content Area -->
    <main class="content">
      {#if view === "metadata"}
        <MetadataComponent />
      {:else if view === "registration"}
        <RegistrationComponent />
      {:else if view === "split"}
        <SplitComponent />
        <!-- <div class="grid">
          <SearchSkinnyComponent />
          {#if isRegistered}
            <ResultsSkinnyComponent />
          {:else}
            <RegisterComponent  />
          {/if}
        </div> -->
      {:else if view === "search"}
        <SearchComponent />
      {:else if view === "results"}
        <ResultsComponent />
      {/if}
    </main>
  </div>
{/if}
{#if $showPopup}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="popup-overlay" on:click={() => ($showPopup = false)}>
    {#if $Popup === "server"}
      <ServerConnectPopup />
    {:else if $Popup === "searchFolder"}
      <SearchFolderPopup />
    {:else if $Popup === "search"}
      <SearchPopup />
    {:else if $Popup === "metadata"}
      <MetadataPopup />
    {:else if $Popup === "remove"}
      <RemovePopup />
    {/if}
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

  .popup-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.7);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }
</style>
