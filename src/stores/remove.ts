import { invoke } from "@tauri-apps/api/core";
import { message } from "@tauri-apps/plugin-dialog";
import { preferencesStore } from "./preferences";
import { filteredItemsStore, selectedItemsStore } from "./results";
import { setDatabase } from "./database";
import { showSearchView } from "./menu";
import type { FileRecord } from "./types";
import { get } from "svelte/store";

/**
 * Interface for a dual mono file
 */
interface DualMonoFile {
  id: number;
  path: string;
}

/**
 * Confirms with the user before removing files
 */
export async function confirmRemovalDialog(): Promise<boolean> {
  // Implementation may vary based on your dialog needs
  const confirmed = await message("Are you sure you want to remove these records?", {
    title: "Confirm Removal",
    type: "warning",
    okLabel: "Remove",
    cancelLabel: "Cancel",
  });
  return confirmed;
}

/**
 * Removes all filtered records that aren't marked with "Keep" 
 */
export async function removeFilteredRecords(): Promise<boolean> {
  const filteredItems = get(filteredItemsStore);
  const preferences = get(preferencesStore);
  let processing = false;
  
  const idsToRemove = filteredItems
    .filter((item) => !item.algorithm.includes("Keep"))
    .map((item) => item.id);
  
  const filesToRemove = filteredItems
    .filter((item) => !item.algorithm.includes("Keep"))
    .map((item) => item.path + "/" + item.filename);

  const dualMono = filteredItems
    .filter((item) => item.algorithm.includes("DualMono"))
    .map((item) => ({ id: item.id, path: item.path + "/" + item.filename }));

  if (idsToRemove.length > 0 || dualMono.length > 0) {
    if (!(await confirmRemovalDialog())) return false;
    
    processing = true;
    try {
      const updatedDb = await invoke<string>("remove_records", {
        records: idsToRemove,
        clone: preferences.safety_db,
        cloneTag: preferences.safety_db_tag,
        delete: preferences.erase_files,
        files: filesToRemove,
        dualMono: dualMono,
        stripDualMono: preferences.strip_dual_mono,
      });

      if (dualMono.length > 0 && preferences.strip_dual_mono) {
        await message(
          "Dual Mono files converted to Mono!\n\nRecords marked as dirty in Soundminer. For safety, open Soundminer and run the following:\n'Database -> Show Dirty'\nPress: 'CMD + A' to select all\n'Database -> Embed Selected'\n'Database -> Rebuild Waveforms for Selected'"
        );
      }
      
      console.log("Successfully removed records with IDs:", idsToRemove);
      processing = false;
      setDatabase(updatedDb, false);
      showSearchView();
      return true;
    } catch (error) {
      console.error("Error removing records:", error);
      processing = false;
      return false;
    }
  } else {
    console.log("No records to remove");
    await message("No records to remove!");
    return false;
  }
}

/**
 * Removes selected records that aren't marked with "Keep"
 */
export async function removeSelectedRecords(): Promise<boolean> {
  const filteredItems = get(filteredItemsStore);
  const selectedItems = get(selectedItemsStore);
  const preferences = get(preferencesStore);
  let processing = false;
  
  const idsToRemove = filteredItems
    .filter(
      (item) => !item.algorithm.includes("Keep") && selectedItems.has(item.id)
    )
    .map((item) => item.id);
  
  const filesToRemove = filteredItems
    .filter(
      (item) => !item.algorithm.includes("Keep") && selectedItems.has(item.id)
    )
    .map((item) => item.path + "/" + item.filename);

  const dualMono = filteredItems
    .filter(
      (item) =>
        item.algorithm.includes("DualMono") && selectedItems.has(item.id)
    )
    .map((item) => ({ id: item.id, path: item.path + "/" + item.filename }));

  if (idsToRemove.length > 0 || dualMono.length > 0) {
    if (!(await confirmRemovalDialog())) return false;
    
    processing = true;
    try {
      const updatedDb = await invoke<string>("remove_records", {
        records: idsToRemove,
        clone: preferences.safety_db,
        cloneTag: preferences.safety_db_tag,
        delete: preferences.erase_files,
        files: filesToRemove,
        dualMono: dualMono,
        stripDualMono: preferences.strip_dual_mono,
      });

      if (dualMono.length > 0 && preferences.strip_dual_mono) {
        await message(
          "Dual Mono files converted to Mono!\n\nRecords marked as dirty in Soundminer. For safety, open Soundminer and run the following:\n'Database -> Show Dirty'\nPress: 'CMD + A' to select all\n'Database -> Embed Selected'\n'Database -> Rebuild Waveforms for Selected'"
        );
      }
      
      console.log("Successfully removed selected records with IDs:", idsToRemove);
      processing = false;
      setDatabase(updatedDb, false);
      showSearchView();
      return true;
    } catch (error) {
      console.error("Error removing records:", error);
      processing = false;
      return false;
    }
  } else {
    console.log("No records to remove");
    await message("No selected records to remove!");
    return false;
  }
}

/**
 * Track an item to be removed (like a checkbox toggle)
 * If your application has specific UI for tracking items to remove
 */
export function toggleRemoveCheck(item: FileRecord): void {
  // This function would depend on how your UI tracks items to remove
  // For example, it might toggle a checkbox or visual indicator
  // Implement based on your existing removeCheck function
}