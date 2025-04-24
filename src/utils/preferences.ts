import { Window } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";
import { preferencesStore, defaultPreferences, } from '../store';


export async function togglePreferencesWindow() {
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
      });

      // Listen for console logs from preferences window
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
  }

  
