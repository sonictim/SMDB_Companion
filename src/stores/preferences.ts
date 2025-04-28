console.log('Loading module:', 'preferences.ts');  // Add to each file
// src/stores/preferences.ts
import type { Preferences } from './types';
import { defaultColors, terminalColors, applyColors } from './colors';
import { defaultAlgorithms } from './algorithms';
import { createLocalStore } from './utils';

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


const storedPreferences = localStorage.getItem('preferences');
let initialPreferences: Preferences;
try {
    const parsedPreferences = storedPreferences ? JSON.parse(storedPreferences) : null;

    // Ensure algorithms are always present, even if not in stored preferences
    initialPreferences = parsedPreferences && Object.keys(parsedPreferences).length > 0
        ? {
            ...defaultPreferences, // ensures all keys are present
            ...parsedPreferences,
            algorithms: Array.isArray(parsedPreferences.algorithms)
                ? parsedPreferences.algorithms
                : defaultAlgorithms
        }
        : defaultPreferences;
} catch (e) {
    console.error('Error loading preferences:', e);
    initialPreferences = defaultPreferences;
}

export const preferencesStore = createLocalStore<Preferences>('preferences', initialPreferences);

export function resetPreferences() {
    preferencesStore.set({ ...defaultPreferences });
}

