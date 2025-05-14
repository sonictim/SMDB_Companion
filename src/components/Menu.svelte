<script lang="ts">
  import {
    Menu,
    PredefinedMenuItem,
    Submenu,
    CheckMenuItem,
  } from "@tauri-apps/api/menu";
  import { onMount, onDestroy } from "svelte";
  import { preferencesStore } from "../stores/preferences";
  import { presetsStore } from "../stores/presets";
  import { openDatabase, closeDatabase } from "../stores/database";
  import { loadPreset } from "../stores/presets";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { Window } from "@tauri-apps/api/window";
  import { openUrl } from "@tauri-apps/plugin-opener";

  // Use reactive store references with $ prefix
  $: algorithms = $preferencesStore?.algorithms || [];
  $: presets = $presetsStore || [];

  let menuInitialized = false;

  // Set up the menu when component mounts
  onMount(async () => {
    await setupMenu();
    menuInitialized = true;
  });

  // Watch for changes in stores and update menu
  $: if (menuInitialized && ($preferencesStore || $presetsStore)) {
    setupMenu();
  }

  async function setupMenu() {
    try {
      console.log("Setting up application menu");

      // Create common menu items
      const copy = await PredefinedMenuItem.new({
        text: "Copy",
        item: "Copy",
      });

      const separator = await PredefinedMenuItem.new({
        text: "separator",
        item: "Separator",
      });

      // Other predefined items here...

      // Build algorithm menu - will automatically update when algorithms change
      const algoMenu = await Submenu.new({
        text: "Algorithms",

        items: await Promise.all(
          algorithms.map(async (algo) => {
            return await CheckMenuItem.new({
              id: algo.id,
              text: algo.name,
              checked: algo.enabled,
              action: () => {
                // Toggle algorithm state
                preferencesStore.update((prefs) => {
                  if (!prefs || !prefs.algorithms) {
                    return prefs || {};
                  }

                  const updatedAlgorithms = prefs.algorithms.map((a) =>
                    a.id === algo.id ? { ...a, enabled: !a.enabled } : a
                  );
                  return { ...prefs, algorithms: updatedAlgorithms };
                });
              },
            });
          })
        ),
      });

      // Presets menu - automatically updates when presets change
      const presetItems = presets.map((preset) => ({
        id: preset.name,
        text: preset.name,
        action: async () => await loadPreset(preset.name),
      }));

      const loadPresetMenu = await Submenu.new({
        text: "Load Preset",
        items: presetItems,
      });

      // App menu
      const appMenu = await Submenu.new({
        text: "App",
        items: [
          await PredefinedMenuItem.new({
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
          }),
          separator,
          {
            id: "settings",
            text: "Settings...",
            action: togglePreferencesWindow,
          },
          separator,
          await PredefinedMenuItem.new({
            text: "Services",
            item: "Services",
          }),
          separator,
          await PredefinedMenuItem.new({
            text: "Hide",
            item: "Hide",
          }),
          await PredefinedMenuItem.new({
            text: "Hide Others",
            item: "HideOthers",
          }),
          await PredefinedMenuItem.new({
            text: "Show All",
            item: "ShowAll",
          }),
          separator,
          await PredefinedMenuItem.new({
            text: "Quit",
            item: "Quit",
          }),
        ],
      });

      // File menu
      const fileMenu = await Submenu.new({
        text: "File",
        items: [
          {
            id: "open",
            text: "Open Database",
            action: () => openDatabase(false),
          },
          { id: "close", text: "Close Database", action: closeDatabase },
          separator,
          loadPresetMenu,
          separator,
          await PredefinedMenuItem.new({
            text: "Close Window",
            item: "CloseWindow",
          }),
        ],
      });

      // Other menus...

      // Create and set the app menu
      const menu = await Menu.new({
        items: [appMenu, fileMenu /* other menus */],
      });

      await menu.setAsAppMenu();
    } catch (error) {
      console.error("Error setting up menu:", error);
    }
  }

  // Toggle preferences window function
  export async function togglePreferencesWindow() {
    try {
      const preferencesWindow = await Window.getByLabel("preferences");

      if (preferencesWindow) {
        const visible = await preferencesWindow.isVisible();

        if (visible) {
          const wasFocused = await preferencesWindow.isFocused();

          if (wasFocused) {
            await preferencesWindow.hide();
          } else {
            await preferencesWindow.setFocus();
          }
        } else {
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
          focus: true,
          transparent: false,
          decorations: true,
        });

        // Event handling for window creation
        appWindow.once("tauri://created", function () {
          console.log("Preferences window created!");
        });

        appWindow.once("tauri://error", function (e) {
          console.error("Error creating preferences window:", e);
        });
      }
    } catch (error) {
      console.error("Error toggling preferences window:", error);
    }
  }

  // Function is already exported above - no need to re-export
</script>
