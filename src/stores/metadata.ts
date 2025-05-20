console.log('Loading module:', 'metadata.ts');  // Add to each file


import type { Metadata, FileRecord } from './types';
import {createSessionStore } from './utils';
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { preferencesStore } from "./preferences";
import { resultsStore, clearResults } from "./results";
import { showResultsView, showSearchView, isRemove } from "./menu";
import { ask } from "@tauri-apps/plugin-dialog";






export const metadataDefault = { 
    find: '', 
    replace: '', 
    column: 'FilePath', 
    case_sensitive: false, 
    mark_dirty: true };


let initialMetadata: Metadata;
try {
    const storedMetadata = sessionStorage.getItem('metadata');
    initialMetadata = storedMetadata ? JSON.parse(storedMetadata) : metadataDefault;
} catch (e) {
    console.error('Error loading metadata:', e);
    initialMetadata = metadataDefault;
}

export const metadataStore = createSessionStore<Metadata>('metadata', initialMetadata);

export async function findMetadata() {
    isRemove.set(false);
    const metaValue = get(metadataStore);
    console.log(
      `Finding: ${metaValue.find}, Replacing: ${metaValue.replace}, Case Sensitive: ${metaValue.case_sensitive}, Column: ${metaValue.column}`
    );

    await invoke<FileRecord[]>("find", {
      find: metaValue.find,
      column: metaValue.column,
      caseSensitive: metaValue.case_sensitive,
      pref: get(preferencesStore),
    })
      .then((result) => {
        console.log(result);
        resultsStore.set(result); // ✅ Store the results in session storage
      })
      .catch((error) => console.error(error));
    showResultsView();
  }


  export async function replaceMetadata() {
    const confirmed = await ask("Are you sure? This is NOT undoable", {
      title: "⚠️ Confirm Replace",
      kind: "warning",
      okLabel: "Yes",
      cancelLabel: "Cancel",
    });
    const metadata = get(metadataStore);

    if (confirmed && metadata.find && metadata.replace) {
      await invoke("replace_metadata", {
        data: metadata,
      })
        .then(() => {
          console.log("Successfully replaced metadata");
          metadata.find = "";
          metadata.replace = "";
          clearResults();
          showSearchView;
        })
        .catch((error) => {
          console.error("Error replacing metadata:", error);
        });
    }
  }

  export function toggleMarkDirty() {
    metadataStore.update((p) => ({
      ...p,
      mark_dirty: !p.mark_dirty,
    }));
  }