console.log('Loading module:', 'menu.ts');

import { 
  Menu, 
  PredefinedMenuItem, 
  Submenu,
  CheckMenuItem,
} from "@tauri-apps/api/menu";
import { createLocalStore, createSessionStore,  } from "./utils";
import { openUrl } from "@tauri-apps/plugin-opener";
import { writable, get } from 'svelte/store';
import { preferencesStore, toggle_ignore_filetype, toggle_remove_records_from, updateEraseFiles, toggle_fetch_waveforms, toggle_store_waveforms, toggle_strip_dual_mono, updateWaveformSearchType } from './preferences';
import { presetsStore } from './presets';
import { openDatabase, closeDatabase, recentDbStore, setDatabase, databaseStore, openDbFolder, clearAllFingerprints, clearSelectedFingerprints } from './database';
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Window } from '@tauri-apps/api/window';
import { loadPreset } from './presets';
import { hotkeysStore, defaultHotKeys, getHotkey } from './hotkeys';
import {removeRecords} from './remove';
  import {
    resultsStore,
    saveStore,
    filteredItemsStore,
    selectedItemsStore,
    currentFilterStore,
    enableSelectionsStore,
    toggleEnableSelections,
    clearSelected,
    invertSelected,
    toggleSelect,
    toggleChecked,
    checkSelected,
    uncheckSelected,
    toggleChecksSelected,
    getTotalChecks,
    updateCurrentFilter,
    filterItems,
    filtersStore,
    manualFiltersStore,
    revealSelectedFiles,
    saveResultsToStore,
    loadResultsFromStore,
  } from "../stores/results";
    import {
    showStatus,
    searchProgressStore,
    initializeSearchListeners,
    toggleSearch, // Import the moved functions
    search,
    cancelSearch,
  } from "../stores/status";
import { show } from "@tauri-apps/api/app";

const DEBUG_MODE = import.meta.env.DEV || false;


  export const showPopup = writable(false);
  export const Popup = writable("search");


  export function ServerPopup() {
     Popup.set("server");
     showPopup.set(true);
  }
  export function MetadataPopup() {
    Popup.set("metadata");
    showPopup.set(true);
  }
  export function SearchPopup() {
    Popup.set("search");
    showPopup.set(true);
  }
  export function SearchFolderPopup() {
    Popup.set("searchFolder");
    showPopup.set(true);
  }
  export function RemovePopup() {
    Popup.set("remove");
    showPopup.set(true);
  }

  export function clearPopups() {
    showPopup.set(false);
  }



export async function refreshMenu() {
  try {
    await setupMenu();
    return true;
  } catch (error) {
    console.error("Error refreshing menu:", error);
    return false;
  }
}

export async function initializeMenu() {
  try {
    menuStore.update(state => ({ ...state, isReady: false, error: null }));
    await setupMenu();
    
    const prefUnsubscribe = preferencesStore.subscribe(async () => {
      await setupMenu();
    });
    
    const recentDbUnsubscribe = recentDbStore.subscribe(async () => {
      await setupMenu();
    });
    
    const presetsUnsubscribe = presetsStore.subscribe(async () => {
      await setupMenu();
    });
    
    const hotkeysUnsubscribe = hotkeysStore.subscribe(async () => {
      await setupMenu();
    });
    
    // Listen for explicit hotkey change events
    const { listen } = await import('@tauri-apps/api/event');
    const hotkeyListener = await listen('hotkey-change', async () => {
      console.log('Received hotkey change event, refreshing menu');
      await setupMenu();
    });
    
    // Store unsubscribe functions to clean up later if needed
    menuStore.update(state => ({ 
      ...state, 
      isReady: true,
      // Combine all unsubscribe functions
      unsubscribe: () => {
        prefUnsubscribe();
        recentDbUnsubscribe();
        presetsUnsubscribe();
        hotkeysUnsubscribe();
        hotkeyListener(); // Unsubscribe from the event listener when needed
      }
    }));
    
    return true;
  } catch (error) {
    console.error("Error initializing menu:", error);
    menuStore.update(state => ({ ...state, isReady: false, error: String(error) }));
    return false;
  }
}



// Create a store for menu state if needed
export const menuStore = writable({
  isReady: false,
  error: null as string | null,
  unsubscribe: null as (() => void) | null
});

/**
 * Toggle the preferences window
 */
export async function togglePreferencesWindow() {
  try {
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
        transparent: false,
        decorations: true,
      });

      // Listen for console logs from preferences window
      const { listen } = await import('@tauri-apps/api/event');
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
  } catch (error) {
    console.error("Error toggling preferences window:", error);
    menuStore.update(state => ({ ...state, error: String(error) }));
  }
}

/**
 * Open save dialog window
 */
export async function openSaveDialog() {
  try {
    const { listen } = await import('@tauri-apps/api/event');
    
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
      focus: true,
    });

    // Listen for console logs from preferences window
    await listen("preferences-log", (event: any) => {
      console.log("Save Window:", event.payload);
    });

    // Add error handling for window events
    appWindow.once("tauri://created", function () {
      console.log("Save window created!");
    });

    appWindow.once("tauri://error", function (e) {
      console.error("Error creating Save window:", e);
      menuStore.update(state => ({ ...state, error: String(e) }));
    });
  } catch (error) {
    console.error("Error opening save dialog:", error);
    menuStore.update(state => ({ ...state, error: String(error) }));
  }
}

export async function openPurchaseLink() {
  await openUrl("https://buy.stripe.com/9AQcPw4D0dFycSYaEE");
}


export async function openManual() {
  await openUrl("https://smdbc.com/manual.php");
}


export async function openLicenseRecovery() {
  await openUrl("https://smdbc.com/recover-key.php");
}

export function toggleSearchBar() {
  const currentView = get(viewStore);
  if (currentView !== "split") {
    viewStore.set("split");
  }
  else 
    viewStore.set("results");
  }
export function toggleNoFrills() {
  const currentView = get(viewStore);
  if (currentView !== "nofrills") {
    viewStore.set("nofrills");
  }
  else 
    viewStore.set("results");
  }




async function setupMenu() {
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
      },
    },
  });

  const filters = get(filtersStore);
  const manualFilters = get(manualFiltersStore);
  const currentFilter = get(currentFilterStore);

  const filtersMenu = await Submenu.new({
    text: "Results Filter",
    items: [
      // ...manualFilters.map((filter) => ({
      //   id: filter.id,
      //   text: filter.name,
      //   checked: filter.id === currentFilter,
      //   action: () => { 
      //     updateCurrentFilter(filter.id);
      //   }
      // })),
      // separator,

      ...filters.map((filter) => {
        if (filter.id === "spacer") return separator;
        return {
          id: filter.id,
          text: filter.name,
          checked: filter.id === currentFilter,
          action: async () => {
            // Update the store
            updateCurrentFilter(filter.id);
            
            // Force menu refresh to ensure only one item is checked
            await setupMenu();
          }
        };



      })


    ]
  });


 const optionsMenu = await Submenu.new({
    text: "Options",
    items: [
      await CheckMenuItem.new({
        id: "ignore-filetypes",
        text: "Ignore File Types",
        checked: get(preferencesStore).ignore_filetype,
        action: async () => {await toggle_ignore_filetype()},

      }),
      separator,
      await CheckMenuItem.new({
        id: "safety-database",
        text: "Create Safety Database",
        checked: get(preferencesStore).safety_db,
        action: async () => {await toggle_remove_records_from()},

      }),
      await Submenu.new({
        text: "Audio Files",
        items: [
          await CheckMenuItem.new({
            id: "keep-audio-files",
            text: "Keep on Disk",
            checked: get(preferencesStore).erase_files === "Keep",
            action: async () => {await 
              updateEraseFiles("Keep");
            },
          }),
          await CheckMenuItem.new({
            id: "archive-audio-files",
            text: "Move to Archive Folder",
            checked: get(preferencesStore).erase_files === "Archive",
            action: async () => {await 
              updateEraseFiles("Archive");
            },
          }),
          await CheckMenuItem.new({
            id: "trash-audio-files",
            text: "Move to Trash",
            checked: get(preferencesStore).erase_files === "Trash",
            action: async () => {await updateEraseFiles("Trash")},
          }),
          await CheckMenuItem.new({
            id: "remove-audio-files",
            text: "Permanently Delete",
            checked: get(preferencesStore).erase_files === "Delete",
            action: async () => {await updateEraseFiles("Delete")}, 
          }),
        ]



      }),
      separator,
      await CheckMenuItem.new({
        id: "strip-dual-mono",
        text: "Strip Dual Mono",
        checked: get(preferencesStore).strip_dual_mono,
        action: async () => {await toggle_strip_dual_mono()},

      }),
      separator,
      await Submenu.new({
        text: "Audio Content Comparison",
        items: [
          await CheckMenuItem.new({
            id: "exact",
            text: "Exact Match",
            checked: get(preferencesStore).waveform_search_type=== "Exact",
            action: async () => {await updateWaveformSearchType("Exact")},
          }),
          await CheckMenuItem.new({
            id: "similarity",
            text: `Relative Match: ${get(preferencesStore).similarity_threshold}%`,
            checked: get(preferencesStore).waveform_search_type === "Similar",
            action: async () => {await updateWaveformSearchType("Similar")},
          }),

        ]



      }),
      await Submenu.new({
        text: "Audio Fingerprints",
        items: [
          await CheckMenuItem.new({
            id: "save-audio-fingerprints",
            text: "Save to Database",
            checked: get(preferencesStore).store_waveforms,
            action: async () => {await toggle_store_waveforms()},
          }),
          await CheckMenuItem.new({
            id: "fetch-audio-fingerprints",
            text: "Fetch from Database",
            checked: get(preferencesStore).fetch_waveforms,
            action: async () => {await toggle_fetch_waveforms()},
          }),
          separator,
          {
            id: "clear-fingerprints",
            text: "Clear All Fingerprints",
            action: () => clearAllFingerprints(),
          },
          {
            id: "clear-selected-fingerprints",
            text: "Clear Selected Fingerprints",
            action: () => clearSelectedFingerprints(),
          },

        ]



      }),
      separator,
      await CheckMenuItem.new({
        id: "showToolbars",
        text: "Show Toolbars",
        checked: get(preferencesStore).showToolbars,
        accelerator: getHotkey("showToolbars"),
        action: () => {preferencesStore.update(prefs => ({
          ...prefs,
          showToolbars: !prefs.showToolbars
        }))},

      }),




    ]


 });


const prefs = get(preferencesStore);
let enabled = prefs?.algorithms?.some(algo => algo.id === 'basic' && algo.enabled === false) ? false : true;

  
const algoMenu = await Submenu.new({
    text: "Algorithms",
    
    items: await Promise.all((prefs?.algorithms || []).map(async (algo) => {
      return await CheckMenuItem.new({
            id: algo.id,
            text: algo.name,
            checked: algo.enabled,
            enabled: (!(algo.id === 'filename' || algo.id === 'audiosuite') || enabled),
            action: () => {
                algo.enabled = !algo.enabled;
                // Update the preferences store to reflect the change
                preferencesStore.update(prefs => {
                    if (!prefs || !prefs.algorithms) {
                        console.warn("Cannot update algorithms: preferences not properly initialized");
                        return prefs || {};
                    }
                    
                    // Find the algorithm in the preferences and update it
                    const updatedAlgorithms = prefs.algorithms.map(a => 
                        a.id === algo.id ? {...a, enabled: algo.enabled} : a
                    );
                    return {...prefs, algorithms: updatedAlgorithms};
                });
            },
        });
    })),
});


  // Get current presets for the presets menu
  const presets = get(presetsStore);

  // Ensure presets is an array before mapping
  const presetItems = Array.isArray(presets) 
    ? presets.map((preset) => {
        return {
          id: preset.name,
          text: preset.name,
          action: async () => await loadPreset(preset.name),
        };
      })
    : [];

  // Create load preset submenu
  const loadPresetMenu = await Submenu.new({
    text: "Load Preset",
    items: presetItems,
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
        accelerator: getHotkey("settings"),
        action: togglePreferencesWindow,
      },
             separator,
        {
          id: "registration",
          text: "Registration",
          accelerator: getHotkey("showRegistration"),
          action: () => {
            showRegistrationView();
          },
        },
      separator,
      services,
      separator,
      hide,
      hideOthers,
      showAll,
      separator,
      closeWindow,
      quit,
    ],
  });

  let recentdb = get(recentDbStore);

  const fileMenu = await Submenu.new({
    text: "File",
    items: [
      {
        id: "open",
        text: "Open Database",
        accelerator: getHotkey("openDatabase"),
        action: () => openDatabase(false),
      },
      await Submenu.new({
        text: "Open Recent",
        items: recentdb
          .filter(db => db.name !== null && db.url !== null && db.name !== "Select Database")
          .map((db, index) => {
            return {
              id: db.name!,
              text: db.name!,
              // Add accelerator only to the first (most recent) item
              accelerator: index === 0 ? getHotkey("openRecent") : undefined,
              action: async () => {
                await setDatabase(db.url!, false);
              }
            };
          })
      }),
      {
        id: "server",
        text: "Connect to Server",
        accelerator: getHotkey("serverDatabase"),
        action: () => ServerPopup(),
        enabled: DEBUG_MODE,
      },

      { id: "close", 
        text: "Close Database",
        accelerator: getHotkey("closeDatabase"),
        enabled: get(databaseStore) !== null, 
        action: () => closeDatabase() },
      
      
      
      separator,
      {
        id: "saveResults",
        text: "Save Results",
        accelerator: getHotkey("saveResults"),
        action: () => {saveResultsToStore()},
      },
      {
        id: "loadResults",
        text: "Load Results",
        accelerator: getHotkey("loadResults"),
        action: () => {loadResultsFromStore()},
      },

      separator,
      {
        id: "searchDatabase",
        text: "Search Database",
        accelerator: getHotkey("searchDatabase"),
        enabled: !get(showStatus) && get(databaseStore) !== null,
        action: () => {search()},
      },
      {
        id: "cancelSearch",
        text: "Cancel Search",
        accelerator: getHotkey("cancelSearch"),
        enabled: get(showStatus),
        action: () => {cancelSearch()},
      },
      {
        id: "removeRecords",
        text: "Remove Records",
        accelerator: getHotkey("removeRecords"),
        enabled: get(isRemove) && get(databaseStore) !== null,
        action: () => {removeRecords()},



      },
      separator,
      loadPresetMenu,
      separator,
      {
        id: "openFiles",
        text: "Reveal Selected Files",
        accelerator: getHotkey("revealSelectedFiles"),
        enabled: true,
        action: async () => {revealSelectedFiles()},
      },
      {
        id: "openFolder",
        text: "Reveal Database Folder",
        accelerator: getHotkey("revealDatabaseFolder"),
        enabled: true,
        action: async () => {openDbFolder()},
      },
      
      
    ],
  });

  const searchMenu = await Submenu.new({
    text: "Search",
    items: [
    
      {
        id: "databaseSearch",
        text: "Database Search",
        accelerator: getHotkey("showDatabaseSearchWindow"),
        action: () => SearchPopup(),
      },
      {
        id: "fileSearch",
        text: "File System Search",
        accelerator: getHotkey("showFileSystemSearchWindow"),
        action: () => SearchFolderPopup(),
      },
      {
        id: "metadata",
        text: "Find/Replace Metadata",
        accelerator: getHotkey("showMmetadataFindReplaceWindow"),
        action: () => MetadataPopup(),
      },
      separator,
      filtersMenu

    ],
  });


const selectionMenu = await Submenu.new({
    text: "Selection",
    id: "selection",
    items: [
      {
        id: "checkSelected",
        text: "Check Selected",
        accelerator: getHotkey("checkSelected"),
        action: () => {checkSelected()}
      },
      {
        id: "uncheckSelected",
        text: "Uncheck Selected",
        accelerator: getHotkey("uncheckSelected"),
        action: () => {uncheckSelected()}
      },
      {
        id: "toggleSelected",
        text: "Toggle Selected",
        accelerator: getHotkey("toggleSelected"),
        action: () => {toggleChecksSelected()}
      },
      separator,
      {
        id: "invertSelected",
        text: "Invert Selection",
        accelerator: getHotkey("invertSelected"),
        action: () => {invertSelected()}
      },
      {
        id: "clearSelected",
        text: "Clear Selection",
        accelerator: getHotkey("clearSelected"),
        action: () => {clearSelected()}
      },
    
    ]});



      

  const textEditMenu = await Submenu.new({
    text: "Text",
    id: "text",
    items: [
      undo, redo, separator, cut, copy, paste, separator, select_all
    ],
  });
  const editMenu = await Submenu.new({
    text: "Edit",
    id: "edit",
    items: [
       {
        id: "checkSelected",
        text: "Check Selected",
        accelerator: getHotkey("checkSelected"),
        action: () => {checkSelected()}
      },
      {
        id: "uncheckSelected",
        text: "Uncheck Selected",
        accelerator: getHotkey("uncheckSelected"),
        action: () => {uncheckSelected()}
      },
      {
        id: "toggleSelected",
        text: "Toggle Selected",
        accelerator: getHotkey("toggleSelected"),
        action: () => {toggleChecksSelected()}
      },
      separator,
      {
        id: "invertSelected",
        text: "Invert Selection",
        accelerator: getHotkey("invertSelected"),
        action: () => {invertSelected()}
      },
      {
        id: "clearSelected",
        text: "Clear Selection",
        accelerator: getHotkey("clearSelected"),
        action: () => {clearSelected()}
      },
    
      
    
      separator,
     
      textEditMenu,
    ],
  });

  const viewState = get(viewStore);

  const viewMenu = await Submenu.new({
    text: "View",
    id: "view",
    items: [
      minimize, 
      maximize, 
      separator, 
      fullscreen, 
      separator,
      await CheckMenuItem.new({
        id: "toggleSearchBar",
        text: "Toggle Search Bar",
        accelerator: getHotkey("toggleSearchBar"),
        checked: viewState === "split",
        action: toggleSearchBar,
      }),

      await CheckMenuItem.new({
        id: "nofrills-view",
        text: "Toggle No Frills",
        accelerator: getHotkey("toggleNoFrills"),
        checked: viewState === "nofrills",
        action: toggleNoFrills,
      }),

      separator,
      {
        id: "advancedSearch",
        text: "Advanced Search View",
        accelerator: getHotkey("showAdvancedSearchView"),
        action: () => {viewStore.set("search")}
      },
      
    ],
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
        accelerator: getHotkey("helpMenu"),
        action: () => openManual(),
      },
      licenseMenu,
    ],
  });

  const menu = await Menu.new({
    items: [appMenu, fileMenu, editMenu, viewMenu, searchMenu, algoMenu, optionsMenu, helpMenu],
  });

  await menu.setAsAppMenu();
}

// View state store
// export const viewStore = writable("search");
// Modified to check for and reset "results" to "search" on initialization
const initialView = (() => {
  try {
    const storedView = localStorage.getItem("view");
    const parsedView = storedView ? JSON.parse(storedView) : "split";
    // If the stored view is "results", reset to "search"
    return parsedView === "results" ? "split" : parsedView;
  } catch (e) {
    console.error("Error loading view state:", e);
    return "split";
  }
})();

export const viewStore = createLocalStore<string>("view", initialView);
export const isRemove = createSessionStore<boolean>("isRemove", true);
export const isFilesOnly = createSessionStore<boolean>("isFilesOnly", true);

// View state management function
export function showSearchView() {
  viewStore.set("search");
  // Force menu refresh
  setupMenu();
}

export function showResultsView() {
  viewStore.set("results");
  // Force menu refresh
  setupMenu();
}

export function showSplitView() {
  viewStore.set("split");
  // Force menu refresh
  setupMenu();
}

export function showNoFrillsView() {
  viewStore.set("nofrills");
  // Force menu refresh
  setupMenu();
}
export function showRegistrationView() {
  viewStore.set("registration");
  // Force menu refresh
  setupMenu();
}


// export function toggleNoFrillsView() {
//   viewStore.update(state => ({
//     ...state,
//     noFrillsView: !state.noFrillsView
//   }));
// }


// export function toggleSearchView() {

 

//   viewStore.update(state => ({
//     ...state,
//     searchView: state.resultsView ? !state.searchView : true,
//     splitView: false,
//   }));
// }

// export function toggleResultsView() {
//   viewStore.update(state => ({
//     ...state,
//     resultsView: state.searchView ? !state.resultsView : true,
//     splitView: false,
//   }));
// }

// export function toggleSplitView() {
//   viewStore.update(state => ({
//     ...state,
//     splitView: !state.splitView
//   }));
// }


export async function toggleServerWindow() {
  try {
    const serverWindow = await Window.getByLabel("server");

    if (serverWindow) {
      const visible = await serverWindow.isVisible();

      if (visible) {
        // Check focus BEFORE changing it
        const wasFocused = await serverWindow.isFocused();

        if (wasFocused) {
          // If it was already focused, hide it
          await serverWindow.hide();
        } else {
          // If it wasn't focused, just bring it to front
          await serverWindow.setFocus();
        }
      } else {
        // Window exists but isn't visible - show it
        await serverWindow.show();
        await serverWindow.setFocus();
      }
    } else {
      // Window doesn't exist - create it
      const url = `${window.location.origin}/server`;
      console.log("Creating Server Window!");
      const appWindow = new WebviewWindow("server", {
        title: "Connect to Server",
        width: 500,
        height: 500,
        visible: true,
        url: url,
        dragDropEnabled: false,
        devtools: true,
        focus: true, // Ensure it gets focus when created
        transparent: false,
        decorations: true,
      });

      // Listen for console logs from preferences window
      const { listen } = await import('@tauri-apps/api/event');
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
  } catch (error) {
    console.error("Error toggling preferences window:", error);
    menuStore.update(state => ({ ...state, error: String(error) }));
  }
}