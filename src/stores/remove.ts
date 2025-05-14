console.log('Loading module:', 'remove.ts');

import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { message, ask } from '@tauri-apps/plugin-dialog';
import { preferencesStore } from './preferences';
import { filteredItemsStore, selectedItemsStore } from './results';
import { setDatabase } from './database';
import { showSearchView } from './menu';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

export const removeProgressStore = writable(0);
export const removeMessageStore = writable('Initializing...');
export const removeStageStore = writable('');
export const processingStore = writable(false);

// Initialize the event listener for remove status
let unlistenRemoveFn: (() => void) | null = null;

export async function initializeRemoveListeners() {
  if (unlistenRemoveFn) {
    unlistenRemoveFn();
  }

  unlistenRemoveFn = await listen<{
    progress: number;
    message: string;
    stage: string;
  }>('remove-status', (event) => {
    const status = event.payload;
    removeProgressStore.set(status.progress);
    removeMessageStore.set(status.message);
    removeStageStore.set(status.stage);
    console.log(
      `Remove status: ${status.stage} - ${status.progress}% - ${status.message}`
    );
    if (status.stage === 'complete') {
      processingStore.set(false);
    }
  });

  // Return the cleanup function directly
  return function cleanup() {
    if (unlistenRemoveFn) {
      unlistenRemoveFn();
      unlistenRemoveFn = null;
    }
  };
}

export async function confirmDialog() {
  const pref = get(preferencesStore);
  let dbDialog = 'Create Safety Copy';
  if (!pref.safety_db) dbDialog = '❌ Current Database';

  let filesDialog = 'Keep in Place';
  if (pref.erase_files === 'Trash') filesDialog = '⚠️ Move to Trash';
  else if (pref.erase_files === 'Delete')
    filesDialog = '❌ Permanently Delete';

  let dualMonoDialog = 'Leave Unchanged';
  if (pref.strip_dual_mono) dualMonoDialog = '❌ Convert to Mono';

  let warningDialog = '';
  if (
    pref.erase_files === 'Delete' ||
    !pref.safety_db ||
    pref.strip_dual_mono
  ) {
    warningDialog = '\n\n⚠️ Are you sure? This is NOT undoable!';
  }

  let titleDialog = 'Confirm Remove';
  if (pref.erase_files === 'Delete' || !pref.safety_db) {
    titleDialog = 'Confirm Remove';
  }

  let dialog = `Files on Disk: ${filesDialog}\nDatabase: ${dbDialog}\nDualMono Files: ${dualMonoDialog} ${warningDialog}`;

  const confirmed = await ask(dialog, {
    title: titleDialog,
    kind: 'warning',
    okLabel: 'Yes',
    cancelLabel: 'Cancel',
  });

  return confirmed;
}

export async function removeRecords() {
  const filteredItems = get(filteredItemsStore);
  const pref = get(preferencesStore);

  const idsToRemove = filteredItems
    .filter((item) => !item.algorithm.includes('Keep'))
    .map((item) => item.id);

  const filesToRemove = filteredItems
    .filter((item) => !item.algorithm.includes('Keep'))
    .map((item) => item.path + '/' + item.filename);

  const dualMono = filteredItems
    .filter((item) => item.algorithm.includes('DualMono'))
    .map((item) => ({ id: item.id, path: item.path + '/' + item.filename }));

  if (idsToRemove.length > 0 || dualMono.length > 0) {
    if (!(await confirmDialog())) return;
    
    processingStore.set(true);
    
    try {
      const updatedDb = await invoke<string>('remove_records', {
        records: idsToRemove,
        clone: pref.safety_db,
        cloneTag: pref.safety_db_tag,
        delete: pref.erase_files,
        files: filesToRemove,
        dualMono: dualMono,
        stripDualMono: pref.strip_dual_mono,
      });
      
      if (dualMono.length > 0 && pref.strip_dual_mono) {
        await message(
          'Dual Mono files converted to Mono!\n\nRecords marked as dirty in Soundminer. For safety, open Soundminer and run the following:\n\'Database -> Show Dirty\'\nPress: \'CMD + A\' to select all\n\'Database -> Embed Selected\'\n\'Database -> Rebuild Waveforms for Selected\''
        );
      }
      
      console.log('Successfully removed records with IDs:', idsToRemove);
      processingStore.set(false);
      await setDatabase(updatedDb, false);
      showSearchView();
      
    } catch (error) {
      console.error('Error removing records:', error);
      processingStore.set(false);
    }
  } else {
    console.log('No records to remove');
    await message('No records to remove!');
  }
}

export async function removeSelectedRecords() {
  const filteredItems = get(filteredItemsStore);
  const selectedItems = get(selectedItemsStore);
  const pref = get(preferencesStore);

  const idsToRemove = filteredItems
    .filter(
      (item) => !item.algorithm.includes('Keep') && selectedItems.has(item.id)
    )
    .map((item) => item.id);
    
  const filesToRemove = filteredItems
    .filter(
      (item) => !item.algorithm.includes('Keep') && selectedItems.has(item.id)
    )
    .map((item) => item.path + '/' + item.filename);

  const dualMono = filteredItems
    .filter(
      (item) =>
        item.algorithm.includes('DualMono') && selectedItems.has(item.id)
    )
    .map((item) => ({ id: item.id, path: item.path + '/' + item.filename }));

  if (idsToRemove.length > 0 || dualMono.length > 0) {
    if (!(await confirmDialog())) return;
    
    processingStore.set(true);
    
    try {
      const updatedDb = await invoke<string>('remove_records', {
        records: idsToRemove,
        clone: pref.safety_db,
        cloneTag: pref.safety_db_tag,
        delete: pref.erase_files,
        files: filesToRemove,
        dualMono: dualMono,
        stripDualMono: pref.strip_dual_mono,
      });
      
      if (dualMono.length > 0 && pref.strip_dual_mono) {
        await message(
          'Dual Mono files converted to Mono!\n\nRecords marked as dirty in Soundminer. For safety, open Soundminer and run the following:\n\'Database -> Show Dirty\'\nPress: \'CMD + A\' to select all\n\'Database -> Embed Selected\'\n\'Database -> Rebuild Waveforms for Selected\''
        );
      }
      
      console.log('Successfully removed records with IDs:', idsToRemove);
      processingStore.set(false);
      await setDatabase(updatedDb, false);
    } catch (error) {
      console.error('Error removing records:', error);
      processingStore.set(false);
    }
  } else {
    console.log('No records to remove');
    await message('No records to remove!');
  }
}

