  import { invoke } from "@tauri-apps/api/core";

  import { openUrl } from "@tauri-apps/plugin-opener";
  import { get } from 'svelte/store';
import { preferencesStore } from '../store';
    export let dbSize = 0;
      export let activeTab = "search";
      
  export let selectedDb: string | null = null;



   export async function openDatabase(is_compare: boolean = false) {
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
  export async function closeDatabase() {
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

async function getSize() {
    console.log("getting size");
    invoke<number>("get_db_size")
      .then((size) => {
        dbSize = size;
        console.log("get size:", size);
      })
      .catch((error) => console.error(error));
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

  