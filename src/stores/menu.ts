console.log('Loading module:', 'menu.ts');

import { 
  Menu, 
  PredefinedMenuItem, 
  Submenu,
  CheckMenuItem,
} from "@tauri-apps/api/menu";
import { openUrl } from "@tauri-apps/plugin-opener";
import { writable, get } from 'svelte/store';
import { preferencesStore } from './preferences';
import { presetsStore } from './presets';
import { openDatabase, closeDatabase } from './database';
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Window } from '@tauri-apps/api/window';
import { loadPreset } from './presets';


export async function initializeMenu() {
  try {
    menuStore.update(state => ({ ...state, isReady: false, error: null }));
    await setupMenu();
    const unsubscribe = preferencesStore.subscribe(async (newPrefs) => {
      await setupMenu();
    });
    
    // Store unsubscribe function to clean up later if needed
    menuStore.update(state => ({ 
      ...state, 
      isReady: true,
      unsubscribe 
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


  const prefs = get(preferencesStore);

  
const algoMenu = await Submenu.new({
    text: "Algorithms",
    items: await Promise.all((prefs?.algorithms || []).map(async (algo) => {
      
        return await CheckMenuItem.new({
            id: algo.id,
            text: algo.name,
            checked: algo.enabled,
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
        action: togglePreferencesWindow,
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
    items: [appMenu, fileMenu, editMenu, viewMenu, algoMenu, helpMenu],
  });

  await menu.setAsAppMenu();
}