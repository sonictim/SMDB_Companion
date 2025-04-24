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
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import {
    Menu,
    PredefinedMenuItem,
    Submenu,
    MenuItem,
  } from "@tauri-apps/api/menu";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import SearchComponent from "../components/Search.svelte";
  import SearchSkinnyComponent from "../components/SearchSkinny.svelte";
  import ResultsComponent from "../components/Results.svelte";
  import ResultsSkinnyComponent from "../components/ResultsSkinny.svelte";
  import MetadataComponent from "../components/Metadata.svelte";
  import RegisterComponent from "../components/Register.svelte";
  import RegisterOnlyComponent from "../components/RegisterOnly.svelte";
  import { registrationStore } from "../store";
  import { get } from "svelte/store";
  export let dbSize = 0;
  export let activeTab = "search";
  export let isRemove = true;
  export let isRegistered = false;
  export let selectedDb: string | null = null;
  import type { Preferences } from "../store";
  import { ask, message, save } from "@tauri-apps/plugin-dialog";
  import {
    preferencesStore,
    PresetsStore,
    defaultPreferences,
    defaultAlgorithms,
    // algorithmsStore,
  } from "../store";

  $: presets = $PresetsStore;

  let appInitialized = false;
  let initError: unknown = null;
  let isBeta = false;
  let versionChanges = "";

  function loadPreset(preset: string) {
    // Your existing code
    if (preset === "Default") {
      const defaultPrefs = structuredClone(defaultPreferences);
      preferencesStore.set(defaultPrefs);

      // Apply colors to current window
      applyColors(defaultPrefs.colors);

      console.log("Default preferences restored");
      return;
    }

    // Existing preset loading logic
    const presetObj = presets.find((p) => p.name === preset);
    if (presetObj) {
      // Your existing code
      const prefCopy = structuredClone(presetObj.pref);
      const defaultPrefs = structuredClone(defaultPreferences);
      const pref = { ...defaultPrefs, ...prefCopy };

      // Set store
      preferencesStore.set(pref);

      // Apply colors to current window
      applyColors(pref.colors || {});
    }
  }

  // Helper function to apply colors to the current document
  function applyColors(colors: Record<string, string>) {
    Object.entries(colors).forEach(([key, value]) => {
      const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
      document.documentElement.style.setProperty(cssVariable, value);
    });
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

      // Run your initialization code
      await checkRegistered();

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

  export async function checkForUpdates(): Promise<{
    latest: string;
    needsUpdate: boolean;
  }> {
    try {
      const response = await fetch(
        "https://smdbc.com/latest.php?token=how-cool-am-i"
      );
      if (!response.ok) throw new Error(`HTTP error: ${response.status}`);
      const response_beta = await fetch(
        "https://smdbc.com/latest_beta.php?token=how-cool-am-i"
      );
      if (!response_beta.ok) throw new Error(`HTTP error: ${response.status}`);
      const response_changelog = await fetch(
        "https://smdbc.com/changelog.php?token=how-cool-am-i"
      );
      if (!response_beta.ok) throw new Error(`HTTP error: ${response.status}`);

      let latestVersion = await response.text();
      const currentVersion: string = await invoke("get_current_version");
      if (!currentVersion.endsWith(".0")) {
        isBeta = true;
        // If the current version ends with .0, use the beta version
        latestVersion = await response_beta.text();
      }

      const fullLog = await response_changelog.text();
      versionChanges = parseChangelog(fullLog, currentVersion);

      console.log("Latest version:", latestVersion);
      console.log("Current version:", currentVersion);

      // Compare versions (you can use semver-parser in frontend)
      const needsUpdate =
        compareVersions(latestVersion.trim(), currentVersion as string) > 0;

      return {
        latest: latestVersion.trim(),
        needsUpdate,
      };
    } catch (error) {
      console.error("Failed to check for updates:", error);
      throw error;
    }
  }

  // Simple version comparison helper
  function compareVersions(v1: string, v2: string): number {
    const parts1 = v1.split(".").map(Number);
    const parts2 = v2.split(".").map(Number);

    for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
      const p1 = parts1[i] || 0;
      const p2 = parts2[i] || 0;
      if (p1 !== p2) return p1 - p2;
    }
    return 0;
  }

  // Helper function to parse changelog and extract relevant changes
  function parseChangelog(changelogJson: string, currentVersion: string) {
    try {
      // Parse the JSON
      const changelog = JSON.parse(changelogJson);

      // Compare versions and collect changes from newer versions
      let relevantChanges = "";

      // Process each version entry
      changelog.forEach(
        (entry: { version: string; date: any; changes: any[] }) => {
          // Compare this version with current version
          if (compareVersions(entry.version, currentVersion) > 0) {
            // This is a newer version, add its changes
            // relevantChanges += `Version ${entry.version} (${entry.date}):\n`;

            // Add each change with bullet points
            if (Array.isArray(entry.changes)) {
              entry.changes.forEach((change) => {
                relevantChanges += `• ${change}\n`;
              });
            }

            // Add extra newline between versions
          }
        }
      );
      relevantChanges += "\n";

      return relevantChanges.trim();
    } catch (error) {
      console.error("Error parsing changelog:", error);
      return "Unable to parse changelog";
    }
  }

  async function fetchColumns() {
    try {
      const columns = await invoke<string[]>("get_columns"); // Fetch from backend
      // Update only the columns in the preferences store while preserving other settings
      preferencesStore.update((currentPrefs) => ({
        ...currentPrefs,
        columns: columns,
      }));
    } catch (error) {
      console.error("Failed to fetch columns:", error);
    }
  }

  // Call the Tauri command to show the hidden window
  async function togglePreferencesWindow() {
    const preferencesWindow = await Window.getByLabel("preferences");

    if (preferencesWindow) {
      const visible = await preferencesWindow.isVisible();

      if (visible) {
        // Check focus BEFORE changing it
        const wasFocused = await preferencesWindow.isFocused();

        if (wasFocused) {
          // If it was already focused, hide it
          await preferencesWindow.hide();
        } else {
          // If it wasn't focused, just bring it to front
          await preferencesWindow.setFocus();
        }
      } else {
        // Window exists but isn't visible - show it
        await preferencesWindow.show();
        await preferencesWindow.setFocus();
      }
    } else {
      // Window doesn't exist - create it
      const url = `${window.location.origin}/preferences`;
      console.log("Creating Pref Window!");
      const appWindow = new WebviewWindow("preferences", {
        title: "Preferences",
        width: 1050,
        height: 800,
        visible: true,
        url: url,
        dragDropEnabled: false,
        devtools: true,
        focus: true, // Ensure it gets focus when created
      });

      // Listen for console logs from preferences window
      await listen("preferences-log", (event: any) => {
        console.log("Preferences Window:", event.payload);
      });

      appWindow.once("tauri://created", function () {
        console.log("Preferences window created!");
      });

      appWindow.once("tauri://error", function (e) {
        console.error("Error creating preferences window:", e);
      });
    }
  }

  async function openSaveDialog() {
    const url = `${window.location.origin}/save`;
    console.log("Creating Save Window!");
    const appWindow = new WebviewWindow("save", {
      title: "Save Preset",
      width: 300,
      height: 200,
      visible: true,
      decorations: true,
      resizable: false,
      alwaysOnTop: false,
      closable: true,
      url: url,
      dragDropEnabled: false,
      devtools: true,
      focus: true, // Ensure it gets focus when created
    });

    // Listen for console logs from preferences window
    await listen("preferences-log", (event: any) => {
      console.log("Save Window:", event.payload);
    });

    appWindow.once("tauri://created", function () {
      console.log("Save window created!");
    });

    appWindow.once("tauri://error", function (e) {
      console.error("Error creating Save window:", e);
    });
  }

  async function checkRegistered() {
    console.log("Checking Registration");
    let reg = get(registrationStore);
    invoke<boolean>("check_reg", { data: reg })
      .then((result) => {
        isRegistered = result;
        console.log("Registration:", result);
      })
      .catch((error) => console.error(error));
    return isRegistered;
  }

  async function getSize() {
    console.log("getting size");
    invoke<number>("get_db_size")
      .then((size) => {
        dbSize = size;
        console.log("get size:", size);
      })
      .catch((error) => console.error(error));
  }

  async function openDatabase(is_compare: boolean = false) {
    await invoke<string>("open_db", { isCompare: is_compare })
      .then((dbPath) => {
        selectedDb = dbPath;
        console.log("db path: ", dbPath);
        if (!(activeTab == "search" || activeTab == "metadata"))
          activeTab = "search";
      })
      .catch((error) => console.error(error));
    // if (dbPath != "") activeTab = 'search';
    await getSize();
    await fetchColumns();
    // await menus();
  }
  async function closeDatabase() {
    await invoke<string>("close_db")
      .then((dbPath) => {
        selectedDb = dbPath;
        console.log("db path: ", dbPath);
        if (!(activeTab == "search" || activeTab == "metadata"))
          activeTab = "search";
      })
      .catch((error) => console.error(error));
    // if (dbPath != "") activeTab = 'search';
    await getSize();
    await fetchColumns();
    // await menus();
  }

  function removeResults() {
    console.log("Remove selected results");
  }

  async function openPurchaseLink() {
    await openUrl("https://buy.stripe.com/9AQcPw4D0dFycSYaEE");
  }
  async function openManual() {
    await openUrl("https://smdbc.com/manual.php");
  }
  async function openLicenseRecovery() {
    await openUrl("https://smdbc.com/recover-key.php");
  }

  async function menu() {
    const copy = await PredefinedMenuItem.new({
      text: "Copy",
      item: "Copy",
    });

    const separator = await PredefinedMenuItem.new({
      text: "separator",
      item: "Separator",
    });

    const undo = await PredefinedMenuItem.new({
      text: "Undo",
      item: "Undo",
    });

    const redo = await PredefinedMenuItem.new({
      text: "Redo",
      item: "Redo",
    });

    const cut = await PredefinedMenuItem.new({
      text: "Cut",
      item: "Cut",
    });

    const paste = await PredefinedMenuItem.new({
      text: "Paste",
      item: "Paste",
    });

    const select_all = await PredefinedMenuItem.new({
      text: "Select All",
      item: "SelectAll",
    });
    const minimize = await PredefinedMenuItem.new({
      text: "Minimize",
      item: "Minimize",
    });
    const maximize = await PredefinedMenuItem.new({
      text: "Maximize",
      item: "Maximize",
    });
    const fullscreen = await PredefinedMenuItem.new({
      text: "Fullscreen",
      item: "Fullscreen",
    });
    const hide = await PredefinedMenuItem.new({
      text: "Hide",
      item: "Hide",
    });
    const hideOthers = await PredefinedMenuItem.new({
      text: "Hide Others",
      item: "HideOthers",
    });
    const showAll = await PredefinedMenuItem.new({
      text: "Show All",
      item: "ShowAll",
    });
    const closeWindow = await PredefinedMenuItem.new({
      text: "Close Window",
      item: "CloseWindow",
    });
    const quit = await PredefinedMenuItem.new({
      text: "Quit",
      item: "Quit",
    });
    const services = await PredefinedMenuItem.new({
      text: "Services",
      item: "Services",
    });
    const about = await PredefinedMenuItem.new({
      text: "About SMDB Companion",
      item: {
        About: {
          name: "SMDB Companion",
          version: "",
          authors: ["Tim Farrell"],
          copyright: "© 2025 Feral Frequencies",
          website: "https://smdbc.com",
          websiteLabel: "SMDB Companion",
          // icon: "icon.png",
        },
      },
    });

    // Rebuild a minimal app menu structure (macOS-style)
    const appMenu = await Submenu.new({
      text: "App",
      items: [
        about,
        separator,
        {
          id: "settings",
          text: "Settings...",
          action: togglePreferencesWindow,
        },
        separator,
        {
          id: "registration",
          text: "Registration",
          action: () => {
            activeTab = "register";
          },
        },
        separator,
        services,
        separator,
        hide,
        hideOthers,
        showAll,
        separator,
        quit,
      ],
    });

    const loadPresetMenu = await Submenu.new({
      text: "Load Preset",
      items: presets.map((preset) => {
        return {
          id: preset.name,
          text: preset.name,
          action: () => loadPreset(preset.name),
        };
      }),
    });

    const fileMenu = await Submenu.new({
      text: "File",
      items: [
        {
          id: "open",
          text: "Open Database",
          action: () => openDatabase(false),
        },
        { id: "close", text: "Close Database", action: () => closeDatabase() },
        separator,
        // {
        //   id: "save",
        //   text: "Save Preset",
        //   action: () => {
        //     openSaveDialog();
        //   },
        // },
        loadPresetMenu,
        separator,
        closeWindow,
      ],
    });

    const editMenu = await Submenu.new({
      text: "Edit",
      id: "edit",
      items: [undo, redo, separator, cut, copy, paste, separator, select_all],
    });

    const viewMenu = await Submenu.new({
      text: "View",
      id: "view",
      items: [minimize, maximize, separator, fullscreen],
    });

    const licenseMenu = await Submenu.new({
      text: "License",
      id: "license",
      items: [
        {
          id: "buy",
          text: "Purchase License",
          action: () => openPurchaseLink(),
        },
        {
          id: "recovery",
          text: "License Recovery",
          action: () => openLicenseRecovery(),
        },
      ],
    });

    const helpMenu = await Submenu.new({
      text: "Help",
      id: "help",
      items: [
        {
          id: "manual",
          text: "User Manual",
          action: () => openManual(),
        },
        licenseMenu,
      ],
    });

    const menu = await Menu.new({
      items: [appMenu, fileMenu, editMenu, viewMenu, helpMenu],
    });

    await menu.setAsAppMenu();
  }

  onMount(menu);

  onMount(checkRegistered);
  onMount(firstOpen);
  onMount(() => {
    checkForUpdates()
      .then(async ({ latest, needsUpdate }) => {
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
            // Command key is pressed
            activeTab = "searchSkinny"; // Or any other action you want
          } else {
            // Normal click
            activeTab = "search";
          }
        }}
      >
        <div class="flex items-center gap-2">
          <SearchIcon size={18} />
          <span>Search</span>
        </div>
      </button>
      <button
        class="nav-link {activeTab === 'results' ? 'active' : ''}"
        on:click={() => (activeTab = "results")}
      >
        <div class="flex items-center gap-2">
          <FilesIcon size={18} />
          <span>Results</span>
        </div>
      </button>
      <!-- <button 
        class="nav-link {activeTab === 'metadata' ? 'active' : ''}"
        on:click={() => activeTab = 'metadata'}
      >
        <div class="flex items-center gap-2">
          <FileText size={18} />
          <span>Metadata</span>
        </div>
      </button> -->
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
    {:else if activeTab === "metadata"}
      <MetadataComponent bind:activeTab bind:isRemove bind:selectedDb />
    {:else if activeTab === "register"}
      <RegisterOnlyComponent bind:isRegistered />
      <!-- <OrderComponent/> -->
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
