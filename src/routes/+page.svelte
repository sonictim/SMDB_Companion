<script lang="ts">
  import {
    Database,
    Search as SearchIcon,
    FileText,
    FilesIcon,
    Settings2,
  } from "lucide-svelte";
  import "../styles.css";
  import { invoke } from "@tauri-apps/api/core";
  import { Window } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";

  // Import menu functions from the new menu.ts module
  import { initializeMenu } from "../stores/menu";

  import SearchComponent from "../components/Search.svelte";
  import SearchSkinnyComponent from "../components/SearchSkinny.svelte";
  import ResultsComponent from "../components/Results.svelte";
  import ResultsSkinnyComponent from "../components/ResultsSkinny.svelte";
  import MetadataComponent from "../components/Metadata.svelte";
  import RegisterComponent from "../components/Register.svelte";
  import RegisterOnlyComponent from "../components/RegisterOnly.svelte";
  import { get } from "svelte/store";
  export let dbSize = 0;
  export let activeTab = "search";
  export let isRemove = true;
  export let isRegistered = false;
  export let selectedDb: string | null = null;
  import { ask, message, save } from "@tauri-apps/plugin-dialog";
  import type { Preferences } from "../stores/types";
  import { preferencesStore } from "../stores/preferences";
  import { presetsStore } from "../stores/presets";
  import { registrationStore } from "../stores/registration";
  import { databaseStore, openDatabase } from "../stores/database";
  import { checkForUpdates } from "../stores/utils";
  import { checkRegistered } from "../stores/registration";
  import { togglePreferencesWindow } from "../stores/menu";

  let appInitialized = false;
  let initError: unknown = null;

  // Use the database store for db state
  $: {
    if ($databaseStore) {
      selectedDb = $databaseStore.path;
      dbSize = $databaseStore.size;
    } else {
      selectedDb = null;
      dbSize = 0;
    }
  }

  // Wrap your onMount in proper error handling
  onMount(async () => {
    try {
      console.log("Starting app initialization");

      // Ensure window is visible and properly positioned
      const mainWindow = Window.getCurrent();
      await mainWindow.show();
      await mainWindow.setFocus();

      // Reset position in case window was created offscreen
      await mainWindow.center();

      console.log("Window setup complete");

      // Initialize the menu
      await initializeMenu();

      // Run your initialization code
      isRegistered = await checkRegistered();

      // Check for updates (but don't block UI on this)
      checkForUpdates().catch((err) => {
        console.error("Update check failed:", err);
      });

      // Mark initialization as complete
      appInitialized = true;
      console.log("App initialization complete");
    } catch (error) {
      console.error("Fatal error during app initialization:", error);
      initError = error;
    }
  });

  let pref: Preferences = get(preferencesStore);

  let preferences;
  preferencesStore.subscribe((value) => {
    preferences = value;
    // Ensure colors object exists before accessing properties
    if (value?.colors) {
      const { colors } = value;
      document.documentElement.style.setProperty(
        "--primary-bg",
        colors.primaryBg ?? "#1a1a1a"
      );
      document.documentElement.style.setProperty(
        "--secondary-bg",
        colors.secondaryBg ?? "#2a2a2a"
      );
      document.documentElement.style.setProperty(
        "--text-color",
        colors.textColor ?? "#ffffff"
      );
      document.documentElement.style.setProperty(
        "--topbar-color",
        colors.topbarColor ?? "#333333"
      );
      document.documentElement.style.setProperty(
        "--accent-color",
        colors.accentColor ?? "#007acc"
      );
      document.documentElement.style.setProperty(
        "--hover-color",
        colors.hoverColor ?? "#2b2b2b"
      );
      document.documentElement.style.setProperty(
        "--warning-color",
        colors.warningColor ?? "#ff4444"
      );
      document.documentElement.style.setProperty(
        "--warning-hover",
        colors.warningHover ?? "#cc0000"
      );
      document.documentElement.style.setProperty(
        "--inactive-color",
        colors.inactiveColor ?? "#666666"
      );
    }
  });

  async function getSize() {
    console.log("getting size");
    invoke<number>("get_db_size")
      .then((size) => {
        dbSize = size;
        console.log("get size:", size);
      })
      .catch((error) => console.error(error));
  }

  function removeResults() {
    console.log("Remove selected results");
  }

  onMount(firstOpen);
  onMount(() => {
    checkForUpdates()
      .then(async ({ latest, needsUpdate, versionChanges, isBeta }) => {
        // needsUpdate = true;
        if (needsUpdate) {
          console.log("Update available:", latest);
          const confirmed = await confirm(
            versionChanges + "\n\nDownload now?",
            {
              title: "Version " + latest + " is available",
              kind: "info",
            }
          );
          if (confirmed) {
            console.log("User confirmed download.");

            if (isBeta) {
              // Open the beta download page in the default browser
              await openUrl("https://smdbc.com/beta.php");
            } else {
              // Open the stable download page in the default browser
              await openUrl("https://smdbc.com/download.php");
            }
          }
        } else {
          console.log("No updates available");
        }
      })
      .catch((err) => {
        console.error("Update check failed:", err);
      });
  });
  async function firstOpen() {
    if (get(preferencesStore).firstOpen) {
      await message(
        "Please be sure to back up your Databases and Audio Files before using this application."
      );
    }

    preferencesStore.update((prefs) => ({
      ...prefs,
      firstOpen: false,
    }));
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

<div class="app-container">
  <!-- Top Bar -->
  <div class="top-bar">
    <div class="top-bar-left">
      <button class="nav-link" on:click={() => openDatabase(false)}>
        <Database size={18} />
        <span style="font-size: 24px;"
          >{selectedDb ? selectedDb : "Select Database"}</span
        >
      </button>
    </div>
    <div class="top-bar-right">
      <button
        class="nav-link {activeTab === 'search' ? 'active' : ''}"
        on:click={(event) => {
          if (event.metaKey) {
            activeTab = "searchSkinny";
          } else {
            activeTab = "search";
          }
        }}
        title="Click for Search, ⌘+Click for Split View"
      >
        <div class="flex items-center gap-2">
          <SearchIcon size={18} />
          <span>Search</span>
        </div>
      </button>
      <button
        class="nav-link {activeTab === 'results' ||
        activeTab === 'resultsSkinny'
          ? 'active'
          : ''}"
        on:click={(event) => {
          if (event.metaKey) {
            activeTab = "resultsSkinny";
          } else {
            activeTab = "results";
          }
        }}
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
      <SearchComponent bind:selectedDb bind:activeTab bind:isRemove />
    {:else if activeTab === "searchSkinny"}
      <div class="grid">
        <SearchSkinnyComponent bind:selectedDb bind:activeTab bind:isRemove />
        {#if isRegistered}
          <ResultsSkinnyComponent
            bind:isRemove
            bind:activeTab
            bind:selectedDb
          />
        {:else}
          <RegisterComponent bind:isRegistered />
        {/if}
      </div>
    {:else if activeTab === "results"}
      {#if isRegistered}
        <ResultsComponent bind:isRemove bind:activeTab bind:selectedDb />
      {:else}
        <RegisterComponent bind:isRegistered />
      {/if}
    {:else if activeTab === "resultsSkinny"}
      {#if isRegistered}
        <ResultsSkinnyComponent bind:isRemove bind:activeTab bind:selectedDb />
      {:else}
        <RegisterComponent bind:isRegistered />
      {/if}
    {:else if activeTab === "metadata"}
      <MetadataComponent bind:activeTab bind:isRemove bind:selectedDb />
    {:else if activeTab === "register"}
      <RegisterOnlyComponent bind:isRegistered />
    {/if}
  </main>
</div>

<style>
  /* .hidden {
    display: none;
  } */

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
