console.log('Loading module:', 'preferences.ts');  // Add to each file
// src/stores/preferences.ts
import type { Preferences } from './types';
import { defaultColors, terminalColors, applyColors } from './colors';
import { defaultAlgorithms } from './algorithms';
import { createLocalStore } from './utils';
import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

// Add version identifier - increment this when you change algorithm order
const PREFERENCES_VERSION = 2;

// Define default preferences
export const defaultPreferences: Preferences = {
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
    version: PREFERENCES_VERSION, // Add version field
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

// Function to ensure algorithms are in the correct order
function ensureCorrectAlgorithmOrder(userAlgorithms: any[]) {
    // If no algorithms, return default
    if (!Array.isArray(userAlgorithms) || userAlgorithms.length === 0) {
        return [...defaultAlgorithms];
    }
    
    // Create a map of user's algorithm settings by id
    const userAlgoMap: Record<string, typeof defaultAlgorithms[0]> = {};
    userAlgorithms.forEach(algo => {
        userAlgoMap[algo.id] = algo;
    });

    // Create new array with default order but user settings
    return defaultAlgorithms.map(defaultAlgo => {
        // If user has this algorithm, preserve their settings
        if (userAlgoMap[defaultAlgo.id]) {
            return {
                ...defaultAlgo,
                ...userAlgoMap[defaultAlgo.id],
                // Keep consistent properties from default like name
                name: defaultAlgo.name
            };
        }
        // Otherwise use the default
        return { ...defaultAlgo };
    });
}

const storedPreferences = localStorage.getItem('preferencesInfo');
let initialPreferences: Preferences;
try {
    const parsedPreferences = storedPreferences ? JSON.parse(storedPreferences) : null;

    // Check if stored preferences exist and have version information
    if (parsedPreferences && Object.keys(parsedPreferences).length > 0) {
        // Ensure algorithms are in correct order
        const orderedAlgorithms = ensureCorrectAlgorithmOrder(parsedPreferences.algorithms);
        
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

export function toggle_ignore_filetype() {
    preferencesStore.update(currentPreferences => ({
        ...currentPreferences,
        ignore_filetype: !currentPreferences.ignore_filetype
    }));
}
export function toggle_remove_records_from() {
    preferencesStore.update(currentPreferences => ({
        ...currentPreferences,
        safety_db: !currentPreferences.safety_db
    }));
}
export function toggle_strip_dual_mono() {
    preferencesStore.update(currentPreferences => ({
        ...currentPreferences,
        strip_dual_mono: !currentPreferences.strip_dual_mono
    }));
}

export function set_keep_audio_files() {
    preferencesStore.update(currentPreferences => ({
        ...currentPreferences,
        erase_files: "Keep"
    }));
}
export function set_trash_audio_files() {
    preferencesStore.update(currentPreferences => ({
        ...currentPreferences,
        erase_files: "Trash"
    }));
}
export function set_remove_audio_files() {
    preferencesStore.update(currentPreferences => ({
        ...currentPreferences,
        erase_files: "Remove"
    }));
}