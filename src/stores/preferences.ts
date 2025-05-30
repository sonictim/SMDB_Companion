console.log('Loading module:', 'preferences.ts');  // Add to each file
// src/stores/preferences.ts
import type { Preferences } from './types';
import { defaultColors, terminalColors, applyColors } from './colors';
import { defaultAlgorithms } from './algorithms';
import { createLocalStore } from './utils';
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { emit } from '@tauri-apps/api/event';
import type { PreservationLogic } from './types';
import { open } from "@tauri-apps/plugin-dialog";


// Add version identifier - increment this when you change algorithm order
const PREFERENCES_VERSION = 3;

// Define default preferences
export const defaultPreferences: Preferences = {
    
    version: PREFERENCES_VERSION, // Add version field
    display_all_records: true,
    match_criteria: ['Filename', 'Channels', 'Duration'],
    ignore_filetype: false,
    safety_db: true,
    safety_db_tag: "thinned",
    erase_files: "Keep",
    strip_dual_mono: false,
    autoselects: [],
    waveform_search_type: "Exact",
    similarity_threshold: 80,
    store_waveforms: true,
    fetch_waveforms: true,
    firstOpen: true,
    showToolbars: true,
    algorithms: defaultAlgorithms,
    batch_size: 1000,
    fontSize: 16, // Default font size in pixels
    safe_folders: [],
    preservation_order: [
        {
            column: "Description",
            operator: "IsNotEmpty",
            variable: "",
        },
        {
            column: "Pathname",
            operator: "DoesNotContain",
            variable: "Audio Files",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "LIBRARIES",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "LIBRARY",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "/LIBRARY",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "LIBRARY/",
        },
        {
            column: "Duration",
            operator: "Largest",
            variable: "",
        },
        {
            column: "Channels",
            operator: "Largest",
            variable: "",
        },
        {
            column: "SampleRate",
            operator: "Largest",
            variable: "",
        },
        {
            column: "BitDepth",
            operator: "Largest",
            variable: "",
        },
        {
            column: "BWDate",
            operator: "Smallest",
            variable: "",
        },
        {
            column: "ScannedDate",
            operator: "Smallest",
            variable: "",
        },

    ],
    tags: ["-1eqa_",
        "-6030_",
        "-7eqa_",
        "-A2sA_",
        "-A44m_",
        "-A44s_",
        "-Alt7S_",
        "-ASMA_",
        "-AVrP_",
        "-AVrT_",
        "-AVSt_",
        "-Altvrb8_",
        "-DUPL_",
        "-DVerb_",
        "-GAIN_",
        "-Gain_",
        "-M2DN_",
        "-NORM_",
        "-NYCT_",
        "-PiSh_",
        "-PnT2_",
        "-PnTPro_",
        "-ProQ2_",
        "-PSh_",
        "-RVRS_",
        "-RX7Cnct_",
        "-spce_",
        "-TCEX_",
        "-TiSh_",
        "-TmShft_",
        "-VariFi_",
        "-VlhllVV_",
        "-VSPD_",
        "-VitmnMn_",
        "-VtmnStr_",
        "-X2mA_",
        "-X2sA_",
        "-XForm_",
        "-Z2N5_",
        "-Z2S5_",
        "-Z4n2_",
        "-ZXN5_"],
    columns: ["AudioFileType", "AuditionLevel", "BWDate", "BWDescription", "BWOriginator", "BWOriginatorRef", "BWTime", "BWTimeStamp", "BitDepth", "Brightness", "CatID", "Category", "CategoryFull", "Category_en", "ChannelLayout", "Channels", "CreationDate", "Description", "Description_en", "Designer", "DesignerInitials", "Duration", "EntryDate", "Era", "FXName", "FilePath", "Filename", "GPSAlt", "GPSLat", "GPSLon", "Index", "Keywords", "LibrarianOnly", "Library", "Location", "LongID", "Manufacturer", "MicPerspective", "Microphone", "ModificationDate", "Notes", "OpenTier", "Pathname", "Popularity", "Rating", "RecMedium", "RecType", "ReleaseDate", "SampleRate", "ScannedDate", "Scene", "ShortID", "Show", "ShowCategory", "ShowDescription", "ShowFXName", "ShowFilename", "ShowLongID", "ShowNotes", "ShowSubCategory", "SoundDesignerOnly", "Source", "SubCategory", "SubCategory_en", "Take", "Tape", "TotalFrames", "TouchedDate", "Track", "TrackTitle", "URL", "Ultrasonics", "UserCategory", "UserComments", "VendorCategory", "Volume", "ixmlCurrentSpeed", "ixmlFileUID", "ixmlMasterSpeed", "ixmlNote", "ixmlOriginalFilename", "ixmlParentFilename", "ixmlParentUID", "ixmlProject", "ixmlSpeedNote", "ixmlTimeCodeFlag", "ixmlTimeCodeRate", "ixmlTrackLayout", "recid"],

    colors: defaultColors,
};

export const TJFPreferences: Preferences = {
    ...defaultPreferences,
    ignore_filetype: true,
    colors: terminalColors,
    match_criteria: ['Filename'],
    autoselects: [".new.", ".wav.", ".mp3.", ".aif.",],
    algorithms: defaultAlgorithms.map(algo => {
        if (algo.id === 'filename') return { ...algo, enabled: true };
        if (algo.id === 'audiosuite') return { ...algo, enabled: true };
        if (algo.id === 'filetags') return { ...algo, enabled: true };
        return algo;
    }),
    tags: [...defaultPreferences.tags, "-Reverse_", "-RING_", ".M.", ".1.", ".1.", ".3.", ".4.", ".5.", ".6.", ".7.", ".8.", ".9.", ".0."],
    preservation_order: [
        {
            column: "Pathname",
            operator: "Contains",
            variable: "TJF RECORDINGS",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "LIBRARIES",
        },
        {
            column: "Pathname",
            operator: "DoesNotContain",
            variable: "SHOWS/Tim Farrell",
        },

        {
            column: "Description",
            operator: "IsNotEmpty",
            variable: "",
        },
        {
            column: "Pathname",
            operator: "DoesNotContain",
            variable: "Audio Files",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "RECORD",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "CREATED SFX",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "CREATED FX",
        },

        {
            column: "Pathname",
            operator: "Contains",
            variable: "LIBRARY",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "/LIBRARY",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "LIBRARY/",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "SIGNATURE",
        },
        {
            column: "Pathname",
            operator: "Contains",
            variable: "PULLS",
        },
        {
            column: "Pathname",
            operator: "DoesNotContain",
            variable: "EDIT",
        },
        {
            column: "Pathname",
            operator: "DoesNotContain",
            variable: "MIX",
        },
        {
            column: "Pathname",
            operator: "DoesNotContain",
            variable: "SESSION",
        },
        {
            column: "Duration",
            operator: "Largest",
            variable: "",
        },
        {
            column: "Channels",
            operator: "Largest",
            variable: "",
        },
        {
            column: "SampleRate",
            operator: "Largest",
            variable: "",
        },
        {
            column: "BitDepth",
            operator: "Largest",
            variable: "",
        },
        {
            column: "AudioFileType",
            operator: "Is",
            variable: "FLAC",
        },
        {
            column: "BWDate",
            operator: "Smallest",
            variable: "",
        },
        {
            column: "ScannedDate",
            operator: "Smallest",
            variable: "",
        },
    ],

}

export function cleanPreferences() {
    addMissingPrefs();
    updateAlgorithmOrder();
}


export function updateAlgorithmOrder() {
    let defAlgo = defaultAlgorithms;
    let userAlgo = get(preferencesStore).algorithms;
    defAlgo.forEach((algo) => {
        const userAlgoIndex = userAlgo.findIndex((user) => user.id === algo.id);
        if (userAlgoIndex !== -1) {
            algo.enabled = userAlgo[userAlgoIndex].enabled;
            algo.name = userAlgo[userAlgoIndex].name;
            algo.db = userAlgo[userAlgoIndex].db;
            algo.min_dur = userAlgo[userAlgoIndex].min_dur;
        }

    });
    updatePreference('algorithms', defAlgo);
}

export function addMissingPrefs() {
    let currentPrefs = get(preferencesStore);
    // Add default preferences only for missing properties
    const updatedPrefs = { ...defaultPreferences, ...currentPrefs };
    preferencesStore.set(updatedPrefs);
    localStorage.setItem('preferencesInfo', JSON.stringify(updatedPrefs));
}

const storedPreferences = localStorage.getItem('preferencesInfo');
let initialPreferences: Preferences;
try {
    const parsedPreferences = storedPreferences ? JSON.parse(storedPreferences) : null;

    // Check if stored preferences exist and have version information
    if (parsedPreferences && Object.keys(parsedPreferences).length > 0) {
        // Ensure algorithms are in correct order
        const orderedAlgorithms = updateAlgorithmOrder;
        
        initialPreferences = {
            ...defaultPreferences, // ensures all keys are present
            ...parsedPreferences,
            algorithms: orderedAlgorithms,
            version: PREFERENCES_VERSION // Force update version
        };
    } else {
        initialPreferences = defaultPreferences;
    }
} catch (e) {
    console.error('Error loading preferences:', e);
    initialPreferences = defaultPreferences;
}

export const preferencesStore = createLocalStore<Preferences>('preferencesInfo', initialPreferences);

export function resetPreferences() {
    preferencesStore.set({ ...defaultPreferences });
}

export async function notifyPreferenceChange() {
  try {
    // Emit an event that all windows can listen for
    await emit('preference-change', {
      timestamp: Date.now(),
    });
    console.log('Emitted preference change event');
  } catch (err) {
    console.error('Failed to emit preference change event:', err);
  }
}

export async function updatePreference(key: string, value: any) {
  preferencesStore.update(p => {
    const updated = { ...p, [key]: value };
    
    // Fix: Use 'preferencesInfo' to match the store's initialization key
    localStorage.setItem('preferencesInfo', JSON.stringify(updated));
    
    return updated;
  });
  
  // Notify other windows
  await notifyPreferenceChange();
}


export async function toggle_ignore_filetype() {
    await updatePreference('ignore_filetype', !get(preferencesStore).ignore_filetype);
}
export async function toggle_remove_records_from() {
    await updatePreference('safety_db', !get(preferencesStore).safety_db);
}
export async function toggle_strip_dual_mono() {
    await updatePreference('strip_dual_mono', !get(preferencesStore).strip_dual_mono);
}

// export function set_keep_audio_files() {
//     updatePreference('erase_files', "Keep");
// }
// export function set_trash_audio_files() {
//     updatePreference('erase_files', "Trash");
// }
// export function set_remove_audio_files() {
//     updatePreference('erase_files', "Delete");
// }

export async function updateEraseFiles(value: string) {
  await updatePreference('erase_files', value);
}
export async function updateWaveformSearchType(value: string) {
  await updatePreference('waveform_search_type', value);
}

 export async function updateSimilarityThreshold(value: number) {
    await updatePreference('similarity_threshold', value);
  }

  export async function toggle_store_waveforms() {
    await updatePreference('store_waveforms', !get(preferencesStore).store_waveforms);
}
  export async function toggle_fetch_waveforms() {
    await updatePreference('fetch_waveforms', !get(preferencesStore).fetch_waveforms);
}

export async function audiosuite_tag_add(value: string) {
    const currentPrefs = get(preferencesStore);
    
    if (value && !currentPrefs.tags.includes(value)) {
        const updatedTags = [...currentPrefs.tags, value];
        updatedTags.sort();
        await updatePreference('tags', updatedTags);
    }
}

export async function filename_tag_add(value: string) {
    const currentPrefs = get(preferencesStore);
    
    if (value && !currentPrefs.autoselects.includes(value)) {
        const updatedAutoselects = [...currentPrefs.autoselects, value];
        updatedAutoselects.sort();
        await updatePreference('autoselects', updatedAutoselects);
    }
}


export async function match_criteria_add(value: string) {
    const currentPrefs = get(preferencesStore);
    
    if (value && !currentPrefs.match_criteria.includes(value)) {
        await updatePreference('match_criteria', [...currentPrefs.match_criteria, value]);
    }
  
}

export async function match_criteria_remove(value: string) {
    const currentPrefs = get(preferencesStore);
    
    if (currentPrefs.match_criteria.includes(value)) {
        await updatePreference('match_criteria', currentPrefs.match_criteria.filter(item => item !== value));
    }
}

export async function audiosuite_tag_remove(value: string) {
    const currentPrefs = get(preferencesStore);
    
    if (currentPrefs.tags.includes(value)) {
        await updatePreference('tags', currentPrefs.tags.filter(item => item !== value));
    }
}

export async function updateFontSize(value: number) {
    // Ensure the value is within the allowed range (12-20) with 1 decimal precision
    const size = parseFloat(
      Math.min(Math.max(value, 12), 20).toFixed(1)
    );
    await updatePreference('fontSize', size);
    
    // Import and use the applyFontSize function from colors.ts
    const { applyFontSize } = await import('./colors');
    await applyFontSize(size);
}
export async function filename_tag_remove(value: string) {
    const currentPrefs = get(preferencesStore);
    
    if (currentPrefs.autoselects.includes(value)) {
        await updatePreference('autoselects', currentPrefs.autoselects.filter(item => item !== value));
    }
}

export async function preservation_order_remove_selected(list: PreservationLogic[]) {

    const currentPrefs = get(preferencesStore);
    
    if (currentPrefs.preservation_order.length > 0) {
        const updatedPreservationOrder = currentPrefs.preservation_order.filter(item => !list.includes(item));
        await updatePreference('preservation_order', updatedPreservationOrder);
    }
}



export async function selected_filename_to_audiosuite_tags(tags: Set<string>, is_move: boolean) {
    tags.forEach(async tag => {
        await audiosuite_tag_add(tag);
        if (is_move) { 
            await filename_tag_remove(tag);
        }
    })
}

export async function selected_audiosuite_to_filename_tags(tags: Set<string>, is_move: boolean) {
    tags.forEach(async tag => {
        await filename_tag_add(tag);
        if (is_move) { 
            await audiosuite_tag_remove(tag);
        }
    })
}

export async function filename_to_audiosuite_tags(is_move: boolean) {
    const currentPrefs = get(preferencesStore);
    const newTags = currentPrefs.autoselects.filter(tag => !currentPrefs.tags.includes(tag));
    
    if (newTags.length > 0) {
        await updatePreference('tags', [...currentPrefs.tags, ...newTags]);
    }
    if (is_move) {  
        await updatePreference('autoselects', []);
    }
}

export async function audiosuite_to_filename_tags(is_move: boolean) {
    const currentPrefs = get(preferencesStore);
    const newAutoselects = currentPrefs.tags.filter(tag => !currentPrefs.autoselects.includes(tag));
    
    if (newAutoselects.length > 0) {
        await updatePreference('autoselects', [...currentPrefs.autoselects, ...newAutoselects]);
    }
    if (is_move) {  
        await updatePreference('tags', []);
    }
}


export async function preservation_order_add(value: PreservationLogic) {
    const currentPrefs = get(preferencesStore);
    
    if (value && !currentPrefs.preservation_order.includes(value)) {
        await updatePreference('preservation_order', [...currentPrefs.preservation_order, value]);
        return true;
    }
    return false;
}

export async function update_preservation_order(newOrder: PreservationLogic[]) {
    await updatePreference('preservation_order', newOrder);
}

export async function update_batch_size(value: number) {
    console.log("📊 [STORE] Updating batch size preference to:", value);
    await updatePreference('batch_size', value);
}

export async function checkThinned(path: string) {
    const p = get(preferencesStore);
    if (path.includes(p.safety_db_tag))
        await updatePreference('safety_db', false);
    else
        await updatePreference('safety_db', true);
    
}

export   async function addSafeFolder() {
    const p = get(preferencesStore);
    let folder = await open({
      multiple: true,
      directory: true,
    });

    if (folder && folder.length > 0) {
        const newSafeFolders = [...p.safe_folders, ...folder];
        await updatePreference('safe_folders', newSafeFolders);
    }
  }

  export async function safe_folder_remove(value: string) {
    const currentPrefs = get(preferencesStore);
    
    if (currentPrefs.safe_folders.includes(value)) {
        await updatePreference('safe_folders', currentPrefs.safe_folders.filter(item => item !== value));
    }
}