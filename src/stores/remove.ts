  import { get } from 'svelte/store';
  import { invoke } from "@tauri-apps/api/core";
  import { message } from "@tauri-apps/plugin-dialog";
import { preferencesStore } from "../stores/preferences";
import {
  resultsStore,
  filteredItemsStore,
  selectedItemsStore,
  removeIdsFromResults,
  updateResultsStore,
} from "../stores/results";
import { metadataStore } from "../stores/metadata";
import { ask } from "@tauri-apps/plugin-dialog";
import { databaseStore, setDatabase, } from "../stores/database";
import { showSearchView } from "../stores/menu";
import { showStatus } from "../stores/status";

  

  export let isRemove: boolean;
  export let selectedDb: string | null = null;



  let idsToRemove: number[] = [];
  let filesToRemove: string[] = [];
  let dualMono: { id: number; path: string }[] = [];





  async function confirmDialog() {
    const pref = get(preferencesStore);
    let dbDialog = "Create Safety Copy";
    if (!pref.safety_db) dbDialog = "❌ Current Database";

    let filesDialog = "Keep in Place";
    if (pref.erase_files === "Trash") filesDialog = "⚠️ Move to Trash";
    else if (pref.erase_files === "Delete") filesDialog = "⛔ Delete Files";

    let dualMonoDialog = "Leave Unchanged";
    if (pref.strip_dual_mono) dualMonoDialog = "❌ Convert to Mono";

    let warningDialog = "";
    if (pref.strip_dual_mono) {
      warningDialog = "(Destructive)";
    }

    let titleDialog = "Confirm Remove";
    if (pref.erase_files === "Delete" || !pref.safety_db) {
      titleDialog = "⚠️ WARNING ⚠️";
    }

    let dialog = `Files on Disk: ${filesDialog}\nDatabase: ${dbDialog}\nDualMono Files: ${dualMonoDialog} ${warningDialog}`;

    const confirmed = await ask(dialog, {
      title: titleDialog,
      cancelLabel: "Cancel",
    });

    return confirmed;
  }



 export async function removeRecords() {
    // let db = get(databaseStore);
    // if (!db) return;
    let filteredItems = get(filteredItemsStore);
    let pref = get(preferencesStore);
    idsToRemove = filteredItems
      .filter((item) => !item.algorithm.includes("Keep"))
      .map((item) => item.id);
    filesToRemove = filteredItems
      .filter((item) => !item.algorithm.includes("Keep"))
      .map((item) => item.path + "/" + item.filename);

    dualMono = filteredItems
      .filter((item) => item.algorithm.includes("DualMono"))
      .map((item) => ({
        id: item.id,
        path: item.path + "/" + item.filename,
      }));

    if (idsToRemove.length > 0 || dualMono.length > 0) {
      const confirmed = await confirmDialog();
      if (confirmed) {
        showStatus.set(true)
        try {
          const updatedDb = await invoke<string>("remove_records", {
            records: idsToRemove,
            clone: pref.safety_db,
            cloneTag: pref.safety_db_tag,
            delete: pref.erase_files,
            files: filesToRemove,
            dualMono: dualMono,
            stripDualMono: pref.strip_dual_mono,
          });
          
          removeIdsFromResults(idsToRemove);
          if (dualMono.length > 0 && pref.strip_dual_mono) {
            await message(
              "Dual Mono files converted to Mono!\n\nRecords marked as dirty in Soundminer. For safety, open Soundminer and run the following:\n'Database -> Show Dirty'\nPress: 'CMD + A' to select all\n'Database -> Embed Selected'\n'Database -> Rebuild Waveforms for Selected'"
            );
          }
          console.log("Successfully removed records with IDs:", idsToRemove);
          showStatus.set(false);
          
          

          const db = get(databaseStore);
          if (db != null && db.path !== updatedDb) await setDatabase(updatedDb, false);
          // let size = db.size - idsToRemove.length;
          // setDbSize(size);
        } catch (error) {
          console.error("Error removing selected records:", error);
          if (String(error).includes("PERMISSION_ERROR:")) {
            await message(String(error) + "\n\nThis operation requires administrator privileges. Please run the app with elevated access or contact your system administrator.", );
        } else {
            await message("Error: " + error);
        }
      } finally {
          showStatus.set(false);
          
        }
      }
    } else {
      console.log("No records to remove");
      await message("No records to remove!");
      showStatus.set(false);
    }
  }

 export async function removeSelectedRecords() {
    let filteredItems = get(filteredItemsStore);
    let selectedItems = get(selectedItemsStore);
    let pref = get(preferencesStore);
    
    idsToRemove = filteredItems
      .filter(
        (item) => !item.algorithm.includes("Keep") && selectedItems.has(item.id)
      )
      .map((item) => item.id);
      
    filesToRemove = filteredItems
      .filter(
        (item) => !item.algorithm.includes("Keep") && selectedItems.has(item.id)
      )
      .map((item) => item.path + "/" + item.filename);

    dualMono = filteredItems
      .filter(
        (item) =>
          selectedItems.has(item.id) && item.algorithm.includes("DualMono")
      )
      .map((item) => ({
        id: item.id,
        path: item.path + "/" + item.filename,
      }));

    if (idsToRemove.length > 0 || dualMono.length > 0) {
      const confirmed = await confirmDialog();
      if (confirmed) {
        showStatus.set(true)
        try {
          const updatedDb = await invoke<string>("remove_records", {
            records: idsToRemove,
            clone: pref.safety_db,
            cloneTag: pref.safety_db_tag,
            delete: pref.erase_files,
            files: filesToRemove,
            dualMono: dualMono,
            stripDualMono: pref.strip_dual_mono,
          });
          removeIdsFromResults(idsToRemove);
          
          if (dualMono.length > 0 && pref.strip_dual_mono) {
            await message(
              "Dual Mono files converted to Mono!\n\nRecords marked as dirty in Soundminer. For safety, open Soundminer and run the following:\n'Database -> Show Dirty'\nPress: 'CMD + A' to select all\n'Database -> Embed Selected'\n'Database -> Rebuild Waveforms for Selected'"
            );
          }
          console.log("Successfully removed records with IDs:", idsToRemove);
          
          showStatus.set(false);
          const db = get(databaseStore);
          if (db != null && db.path !== updatedDb) await setDatabase(updatedDb, false);
        } catch (error) {
          console.error("Error removing selected records:", error);
          if (String(error).includes("PERMISSION_ERROR:")) {
            await message(String(error) + "\n\nThis operation requires administrator privileges. Please run the app with elevated access or contact your system administrator.", );
        } else {
            await message("Error: " + error);
        }
      } finally {
          showStatus.set(false);
        }
      }
    } else {
      console.log("No records to remove");
      await message("No selected records to remove!");
      showStatus.set(false);
    }
  }

 export async function replaceMetadata() {
    let metadata = get(metadataStore);
    const confirmed = await ask("Are you sure? This is NOT undoable", {
      title: "⚠️ Confirm Replace",
      kind: "warning",
      okLabel: "Yes",
      cancelLabel: "Cancel",
    });

    if (confirmed && metadata.find && metadata.replace) {
      showStatus.set(true);
      try {
        await invoke("replace_metadata", {
          data: metadata,
        })
          .then(() => {
            console.log("Successfully replaced metadata");
            metadataStore.update(m => ({
              ...m,
              find: "",
              replace: ""
            }));
            updateResultsStore([]);
          });
      } catch (error) {
        console.error("Error replacing metadata:", error);
        await ask("An error occurred while replacing metadata.");
      } finally {
        showStatus.set(false);
      }
    }
  }

 export function toggleMarkDirty() {
    metadataStore.update((p) => ({
      ...p,
      mark_dirty: !p.mark_dirty,
    }));
  }


