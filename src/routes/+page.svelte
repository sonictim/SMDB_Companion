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
  import { Menu } from "@tauri-apps/api/menu";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import SearchComponent from "../components/Search.svelte";
  import ResultsComponent from "../components/Results.svelte";
  import MetadataComponent from "../components/Metadata.svelte";
  import RegisterComponent from "../components/Register.svelte";
  import { registrationStore } from "../store";
  import { get } from "svelte/store";
  export let dbSize = 0;
  export let activeTab = "search";
  export let isRemove = true;
  export let isRegistered = false;
  export let selectedDb: string | null = null;
  import { preferencesStore } from "../store";
  import type { Preferences } from "../store";

  let pref: Preferences = get(preferencesStore);

  let preferences;
  preferencesStore.subscribe((value) => {
    preferences = value;
    // Ensure colors object exists before accessing properties
    if (value?.colors) {
      const { colors } = value;
      document.documentElement.style.setProperty(
        "--primary-bg",
        colors.primaryBg ?? "#1a1a1a",
      );
      document.documentElement.style.setProperty(
        "--secondary-bg",
        colors.secondaryBg ?? "#2a2a2a",
      );
      document.documentElement.style.setProperty(
        "--text-color",
        colors.textColor ?? "#ffffff",
      );
      document.documentElement.style.setProperty(
        "--topbar-color",
        colors.topbarColor ?? "#333333",
      );
      document.documentElement.style.setProperty(
        "--accent-color",
        colors.accentColor ?? "#007acc",
      );
      document.documentElement.style.setProperty(
        "--hover-color",
        colors.hoverColor ?? "#2b2b2b",
      );
      document.documentElement.style.setProperty(
        "--warning-color",
        colors.warningColor ?? "#ff4444",
      );
      document.documentElement.style.setProperty(
        "--warning-hover",
        colors.warningHover ?? "#cc0000",
      );
      document.documentElement.style.setProperty(
        "--inactive-color",
        colors.inactiveColor ?? "#666666",
      );
    }
  });

  export async function checkForUpdates(): Promise<{
    latest: string;
    needsUpdate: boolean;
  }> {
    try {
      const response = await fetch(
        "https://smdbc.com/latest.php?token=how-cool-am-i",
      );
      if (!response.ok) throw new Error(`HTTP error: ${response.status}`);

      const latestVersion = await response.text();
      const currentVersion = await invoke("get_current_version");

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

  async function menus() {
    const menu = await Menu.new({
      items: [
        {
          id: "Open",
          text: "open",
          action: () => {
            console.log("open pressed");
          },
        },
        {
          id: "Close",
          text: "close",
          action: () => {
            console.log("close pressed");
          },
        },
      ],
    });

    await menu.setAsAppMenu();
  }

  // Call the Tauri command to show the hidden window
  async function togglePreferencesWindow() {
    const preferencesWindow = await Window.getByLabel("preferences");

    if (preferencesWindow) {
      const visible = await preferencesWindow.isVisible();

      if (visible) {
        await preferencesWindow.hide();
      } else {
        await preferencesWindow.show();
      }
    } else {
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

  function removeResults() {
    console.log("Remove selected results");
  }

  onMount(checkRegistered);
  onMount(() => {
    checkForUpdates()
      .then(async ({ latest, needsUpdate }) => {
        // needsUpdate = true;
        if (needsUpdate) {
          console.log("Update available:", latest);
          const confirmed = await confirm(
            "Version " + latest + " is available. Download now?",
            {
              title: "Update Available",
              kind: "info",
            },
          );
          if (confirmed) {
            console.log("User confirmed download.");
            // Open the download page in the default browser
            await openUrl("https://smdbc.com/download.php");
          }
        } else {
          console.log("No updates available");
        }
      })
      .catch((err) => {
        console.error("Update check failed:", err);
      });
  });
</script>

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
        on:click={() => (activeTab = "search")}
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
      <SearchComponent {dbSize} bind:selectedDb bind:activeTab bind:isRemove />
    {:else if activeTab === "results"}
      {#if isRegistered}
        <ResultsComponent
          {removeResults}
          bind:isRemove
          bind:activeTab
          bind:selectedDb
        />
      {:else}
        <RegisterComponent bind:isRegistered />
      {/if}
    {:else if activeTab === "metadata"}
      <MetadataComponent bind:activeTab bind:isRemove bind:selectedDb />
      <!-- <OrderComponent/> -->
    {/if}
  </main>
</div>
