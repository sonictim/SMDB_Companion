import { createLocalStore } from "./utils";
import type { HotKeys } from "./types";
import { get } from "svelte/store";

export const defaultHotKeys: HotKeys = {
  settings: "CmdOrCtrl+,",
  showToolbars: "CmdOrCtrl+T", // Fixed: was just "," before
  showSearchView: "1",
  showResultsView: "2", 
  showSplitView: "3",
  showNoFrillsView: "4",
  showRegistration: "5",
  openDatabase: "CmdOrCtrl+O",
  openRecent: "CmdOrCtrl+Shift+O",
  closeDatabase: "CmdOrCtrl+W",
  searchDatabase: "CmdOrCtrl+Enter",
  cancelSearch: "Esc",
  checkSelected: "C",
  uncheckSelected: "U",
  toggleSelected: "T",
  invertSelected: "I",
  clearSelected: "Backspace",
  helpMenu: "F1"
}

export const hotkeysStore = createLocalStore<HotKeys>('hotkeys', defaultHotKeys);