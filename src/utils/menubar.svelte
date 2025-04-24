<script lang="ts">
  import { onMount } from "svelte";
  import {
    Menu,
    PredefinedMenuItem,
    Submenu,
    MenuItem,
  } from "@tauri-apps/api/menu";
  import {
    preferencesStore,
    PresetsStore,
    defaultPreferences,
    defaultAlgorithms,
    // algorithmsStore,
  } from "../store";
  import { openDatabase, closeDatabase } from "../utils/database";
  import { togglePreferencesWindow } from "../utils/preferences";
  import { openUrl } from "@tauri-apps/plugin-opener";

  $: presets = $PresetsStore;
  export let activeTab: string;

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

  export async function openPurchaseLink() {
    await openUrl("https://buy.stripe.com/9AQcPw4D0dFycSYaEE");
  }
  export async function openManual() {
    await openUrl("https://smdbc.com/manual.php");
  }
  export async function openLicenseRecovery() {
    await openUrl("https://smdbc.com/recover-key.php");
  }

  export function loadPreset(preset: string) {
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
</script>
