use crate::prelude::*;
use metaflac::{Block, Tag};

// Chunk Identifiers
const FMT_CHUNK_ID: &[u8; 4] = b"fmt ";
const DATA_CHUNK_ID: &[u8; 4] = b"data";

#[derive(Debug)]
pub enum Metadata {
    Wav(Vec<MetadataChunk>),
    Flac(metaflac::Tag),
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata::Wav(Vec::new())
    }
}

impl Metadata {
    pub fn get_metadata(path: &Path) -> Self {
        if let Some(path) = path.extension() {
            let p = path.to_str().unwrap_or_default();
            match path.to_str() {
                Some("wav") => {
                    let c = WavCodec;
                    c.extract_metadata_from_file(p).unwrap_or_default()
                }
                Some("flac") => {
                    let c = FlacCodec;
                    c.extract_metadata_from_file(p).unwrap_or_default()
                }
                _ => Metadata::default(),
            }
        } else {
            Metadata::default()
        }
    }
    pub fn set_metadata(&self, p: &str) -> R<()> {
        match self {
            Metadata::Wav(chunks) => {
                let c = WavCodec;
                c.embed_metadata_to_file(p, &Metadata::Wav(chunks.clone()))?;
            }
            Metadata::Flac(tag) => {
                let c = FlacCodec;
                c.embed_metadata_to_file(p, &Metadata::Flac(tag.clone()))?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum MetadataChunk {
    Bext(Vec<u8>),
    IXml(String),
    Soundminer(Vec<u8>), // could later parse this if needed
    ID3(Vec<u8>),        // For MP3 and potentially other formats
    APE(Vec<u8>),        // APE tags used in various formats
    Picture {
        // For album art/images
        mime_type: String,
        description: String,
        data: Vec<u8>,
    },
    TextTag {
        // For simple text-based metadata
        key: String,
        value: String,
    },
    Unknown {
        id: String,
        data: Vec<u8>,
    },
}

impl MetadataChunk {
    pub fn id(&self) -> String {
        match self {
            MetadataChunk::Bext(_) => "bext".to_string(),
            MetadataChunk::IXml(_) => "iXML".to_string(),
            MetadataChunk::Soundminer(_) => "SMED".to_string(),
            MetadataChunk::ID3(_) => "ID3".to_string(),
            MetadataChunk::APE(_) => "APE".to_string(),
            MetadataChunk::Picture { .. } => "Picture".to_string(),
            MetadataChunk::TextTag { key, .. } => key.clone(),
            MetadataChunk::Unknown { id, .. } => id.clone(),
        }
    }
    pub fn data(&self) -> &[u8] {
        match self {
            MetadataChunk::Bext(data) => data,
            MetadataChunk::IXml(data) => data.as_bytes(),
            MetadataChunk::Soundminer(data) => data,
            MetadataChunk::ID3(data) => data,
            MetadataChunk::APE(data) => data,
            MetadataChunk::Picture { data, .. } => data,
            MetadataChunk::TextTag { value, .. } => value.as_bytes(),
            MetadataChunk::Unknown { data, .. } => data,
        }
    }

    pub fn as_text_tags(&self) -> Vec<(String, String)> {
        match self {
            Self::IXml(xml) => {
                let mut tags = Vec::new();
                for line in xml.lines() {
                    if let Some(idx) = line.find('=') {
                        let key = line[0..idx].trim().to_string();
                        let value = line[idx + 1..].trim().to_string();
                        tags.push((key, value));
                    }
                }
                tags
            }
            Self::TextTag { key, value } => {
                vec![(key.clone(), value.clone())]
            }
            _ => Vec::new(),
        }
    }

    pub fn to_format(&self, format: &str) -> Option<MetadataChunk> {
        match (self, format) {
            (Self::IXml(_), "mp3") => {
                let _ = self.as_text_tags();
                Some(Self::ID3(Vec::new()))
            }
            (Self::ID3(_), "wav" | "flac") => Some(Self::IXml(String::new())),
            (
                Self::Picture {
                    mime_type,
                    description,
                    data,
                },
                _,
            ) => Some(Self::Picture {
                mime_type: mime_type.clone(),
                description: description.clone(),
                data: data.clone(),
            }),
            _ => None,
        }
    }

    pub fn new_text_tag(key: &str, value: &str) -> Self {
        Self::TextTag {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    pub fn new_picture(mime_type: &str, description: &str, data: &[u8]) -> Self {
        Self::Picture {
            mime_type: mime_type.to_string(),
            description: description.to_string(),
            data: data.to_vec(),
        }
    }
}

impl WavCodec {
    pub fn extract_metadata_from_file(&self, file_path: &str) -> R<Metadata> {
        println!("extract_file_metadata_chunks - Processing {}", file_path);

        let file = std::fs::File::open(file_path)?;
        let mapped_file = unsafe { MmapOptions::new().map(&file)? };

        // Let's check the channel count in the WAV header before extraction
        if file_path.ends_with(".wav") {
            let mut cursor = Cursor::new(&mapped_file);
            cursor.seek(SeekFrom::Start(22))?; // Position of channel count in WAV header
            let channel_count = cursor.read_u16::<LittleEndian>()?;
            println!(
                "extract_file_metadata_chunks - File has {} channels in header",
                channel_count
            );
        }
        let chunks = self.extract_metadata_chunks(&mapped_file)?;
        Ok(Metadata::Wav(chunks))
    }

    pub fn embed_metadata_to_file(&self, file_path: &str, metadata: &Metadata) -> R<()> {
        let chunks = match metadata {
            Metadata::Wav(chunks) => chunks,
            _ => return Err(anyhow!("Unsupported metadata format")),
        };

        let file = std::fs::File::open(file_path)?;
        let mapped_file = unsafe { MmapOptions::new().map(&file)? };

        // Use mapped_file as &[u8] without loading into memory
        let new_data = self.embed_metadata_chunks(&mapped_file, chunks)?;

        // Format-specific validation - only run for WAV files
        if file_path.ends_with(".wav") {
            let mut cursor = Cursor::new(&new_data);
            cursor.seek(SeekFrom::Start(22))?; // Position of channel count in WAV header
            let channel_count = cursor.read_u16::<LittleEndian>()?;
            println!(
                "embed_file_metadata_chunks - Channel count in output file: {}",
                channel_count
            );
        }

        // Write the data back to the file
        std::fs::write(file_path, new_data)?;
        Ok(())
    }
    fn extract_metadata_chunks(&self, input: &[u8]) -> R<Vec<MetadataChunk>> {
        let mut cursor = Cursor::new(input);

        let mut header = [0u8; 12];
        cursor.read_exact(&mut header)?;

        if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
            return Err(anyhow!("Not a WAV file"));
        }

        let mut chunks = Vec::new();
        while cursor.position() < input.len() as u64 {
            let mut id = [0u8; 4];
            if cursor.read(&mut id)? < 4 {
                break;
            }

            let size = cursor.read_u32::<LittleEndian>()?;

            // Skip the 'data' chunk and 'fmt ' chunk - they're not metadata
            if &id == DATA_CHUNK_ID || &id == FMT_CHUNK_ID {
                cursor.seek(SeekFrom::Current(size as i64 + (size % 2) as i64))?;
                continue;
            }

            let mut data = vec![0u8; size as usize];
            cursor.read_exact(&mut data)?;

            let chunk = match &id {
                b"bext" => MetadataChunk::Bext(data),
                b"iXML" => {
                    let xml = String::from_utf8_lossy(&data).to_string();

                    // Also extract individual text tags for better format conversion
                    for line in xml.lines() {
                        if let Some(idx) = line.find('=') {
                            let key = line[0..idx].trim().to_string();
                            let value = line[idx + 1..].trim().to_string();

                            // Only add if it's a valid key-value pair
                            if !key.is_empty() {
                                chunks.push(MetadataChunk::TextTag { key, value });
                            }
                        }
                    }

                    MetadataChunk::IXml(xml)
                }
                // Recognize ID3 chunk if present in WAV
                b"id3 " | b"ID3 " => MetadataChunk::ID3(data),
                // Picture/album art in WAV
                b"APIC" => {
                    // Try to extract picture metadata
                    if data.len() > 8 {
                        // Simple picture extraction
                        // In a real implementation, you'd parse the APIC structure properly
                        chunks.push(MetadataChunk::Picture {
                            mime_type: "image/jpeg".to_string(), // Default assumption
                            description: "Album Art".to_string(),
                            data: data.clone(),
                        });
                    }

                    // Also keep the raw data
                    MetadataChunk::Unknown {
                        id: "APIC".to_string(),
                        data,
                    }
                }
                b"SMED" | b"SMRD" | b"SMPL" => MetadataChunk::Soundminer(data),
                _ => MetadataChunk::Unknown {
                    id: String::from_utf8_lossy(&id).to_string(),
                    data,
                },
            };

            chunks.push(chunk);

            // Padding: chunks are aligned to even sizes
            if size % 2 == 1 {
                cursor.seek(SeekFrom::Current(1))?;
            }
        }

        Ok(chunks)
    }

    fn embed_metadata_chunks(&self, input: &[u8], chunks: &[MetadataChunk]) -> R<Vec<u8>> {
        let mut cursor = Cursor::new(input);
        let mut output = Cursor::new(Vec::new());

        // Copy the RIFF/WAVE header
        let mut riff_header = [0u8; 12];
        cursor.read_exact(&mut riff_header)?;
        output.write_all(&riff_header)?;

        // Read the original channel count from the input file
        let mut original_cursor = Cursor::new(input);
        original_cursor.seek(SeekFrom::Start(22))?; // Position of channel count in WAV header
        let original_channels = original_cursor.read_u16::<LittleEndian>()?;

        // Group metadata by type for better organization
        let mut bext_chunks = Vec::new();
        let mut ixml_chunks = Vec::new();
        let mut picture_chunks = Vec::new();
        let mut id3_chunks = Vec::new();
        let mut text_tags = Vec::new();
        let mut other_chunks = Vec::new();

        // When reading and writing non-metadata chunks, preserve the original fmt chunk
        let mut fmt_chunk_found = false;

        // First collect all chunks from source audio
        while cursor.position() < input.len() as u64 {
            let mut id = [0u8; 4];
            if cursor.read(&mut id)? < 4 {
                break;
            }

            let size = cursor.read_u32::<LittleEndian>()?;
            let mut data = vec![0u8; size as usize];
            cursor.read_exact(&mut data)?;

            let id_str = String::from_utf8_lossy(&id).to_string();

            // Handle fmt chunk specially to preserve channel count
            if &id == FMT_CHUNK_ID {
                fmt_chunk_found = true;

                // We need to preserve the fmt chunk but ensure it has the correct channel count
                if original_channels == 1 {
                    // For mono files, make sure fmt chunk shows 1 channel
                    // Channel count is at offset 2 in fmt chunk
                    data[2] = 1;
                    data[3] = 0; // Little-endian representation of 1

                    // Update block align and byte rate to match mono format
                    let bits_per_sample = u16::from_le_bytes([data[14], data[15]]);
                    let bytes_per_sample = bits_per_sample / 8;

                    // Block align (offset 12-13) = channels * bytes_per_sample
                    let block_align = bytes_per_sample;
                    data[12] = (block_align & 0xFF) as u8;
                    data[13] = ((block_align >> 8) & 0xFF) as u8;

                    // Byte rate (offset 8-11) = sample_rate * block_align
                    let sample_rate = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                    let byte_rate = sample_rate * block_align as u32;
                    data[8] = (byte_rate & 0xFF) as u8;
                    data[9] = ((byte_rate >> 8) & 0xFF) as u8;
                    data[10] = ((byte_rate >> 16) & 0xFF) as u8;
                    data[11] = ((byte_rate >> 24) & 0xFF) as u8;
                }

                // Write the fmt chunk with potentially updated data
                output.write_all(&id)?;
                output.write_u32::<LittleEndian>(size)?;
                output.write_all(&data)?;

                if size % 2 == 1 {
                    output.write_all(&[0])?; // padding
                }
                continue;
            }

            // Skip known metadata chunks since we'll replace them
            if matches!(
                id_str.as_str(),
                "bext" | "iXML" | "SMED" | "SMRD" | "SMPL" | "id3 " | "ID3 " | "APIC"
            ) {
                if size % 2 == 1 {
                    cursor.seek(SeekFrom::Current(1))?;
                }
                continue;
            }

            // Write other chunks directly to output
            output.write_all(&id)?;
            output.write_u32::<LittleEndian>(size)?;
            output.write_all(&data)?;

            if size % 2 == 1 {
                output.write_all(&[0])?;
            }
        }

        // If the fmt chunk wasn't found in the input file (unlikely), don't proceed
        if !fmt_chunk_found {
            return Err(anyhow!("WAV file missing fmt chunk"));
        }

        // Organize metadata chunks by type
        for chunk in chunks {
            match chunk {
                MetadataChunk::Bext(data) => {
                    // Update channel count in Broadcast WAV extension if necessary
                    let mut bext_data = data.clone();
                    if original_channels == 1 && bext_data.len() >= 356 {
                        // Update channel count in BEXT chunk (at offset 354-355)
                        bext_data[354] = 1;
                        bext_data[355] = 0; // Little-endian representation of 1
                    }
                    bext_chunks.push(MetadataChunk::Bext(bext_data));
                }
                MetadataChunk::IXml(xml) => {
                    // Check for any channel references in iXML that need updating
                    let mut updated_xml = xml.clone();
                    if original_channels == 1 {
                        // Replace any references to "2 channels" or similar with "1 channel"
                        // This is a simplistic approach and might need refinement
                        updated_xml = updated_xml.replace("CHANNELS=2", "CHANNELS=1");
                        updated_xml = updated_xml.replace("channels=2", "channels=1");
                        updated_xml = updated_xml.replace("NumChannels=2", "NumChannels=1");
                    }
                    ixml_chunks.push(MetadataChunk::IXml(updated_xml));
                }
                MetadataChunk::Picture {
                    mime_type,
                    description,
                    data,
                } => picture_chunks.push(MetadataChunk::Picture {
                    mime_type: mime_type.clone(),
                    description: description.clone(),
                    data: data.clone(),
                }),
                MetadataChunk::ID3(data) => id3_chunks.push(MetadataChunk::ID3(data.clone())),
                MetadataChunk::TextTag { key, value } => text_tags.push(MetadataChunk::TextTag {
                    key: key.clone(),
                    value: value.clone(),
                }),
                MetadataChunk::APE(data) => {
                    // APE tags can be handled similarly to ID3
                    other_chunks.push(MetadataChunk::APE(data.clone()));
                }
                MetadataChunk::Soundminer(data) => {
                    other_chunks.push(MetadataChunk::Soundminer(data.clone()))
                }
                MetadataChunk::Unknown { id, data } => other_chunks.push(MetadataChunk::Unknown {
                    id: id.clone(),
                    data: data.clone(),
                }),
            }
        }

        // Consolidate text tags into iXML if no iXML chunk exists
        if ixml_chunks.is_empty() && !text_tags.is_empty() {
            let mut xml = String::new();
            for tag in &text_tags {
                if let MetadataChunk::TextTag { key, value } = tag {
                    xml.push_str(&format!("{}={}\n", key, value));
                }
            }
            if !xml.is_empty() {
                // Create an owned MetadataChunk that's stored directly in the vector
                ixml_chunks.push(MetadataChunk::IXml(xml));
            }
        }

        // Write metadata chunks in order
        // Write bext chunks
        for chunk in &bext_chunks {
            if let MetadataChunk::Bext(data) = chunk {
                write_chunk(&mut output, b"bext", data)?;
            }
        }

        // Write iXML chunks
        for chunk in &ixml_chunks {
            if let MetadataChunk::IXml(xml) = chunk {
                write_chunk(&mut output, b"iXML", xml.as_bytes())?;
            }
        }

        // Write picture chunks
        for chunk in &picture_chunks {
            if let MetadataChunk::Picture { data, .. } = chunk {
                // In WAV, we need to use a custom chunk for pictures
                write_chunk(&mut output, b"APIC", data)?;
            }
        }

        // Write ID3 chunks
        for chunk in &id3_chunks {
            if let MetadataChunk::ID3(data) = chunk {
                write_chunk(&mut output, b"id3 ", data)?;
            }
        }

        // Write Soundminer and other chunks
        for chunk in &other_chunks {
            match chunk {
                MetadataChunk::Soundminer(data) => write_chunk(&mut output, b"SMED", data)?,
                MetadataChunk::Unknown { id, data } => {
                    write_chunk(&mut output, id.as_bytes(), data)?;
                }
                _ => {} // Skip other types we don't handle
            }
        }

        // Update RIFF chunk size
        let final_size = output.position() as u32 - 8;
        let output_data = output.into_inner();
        let mut result_data = output_data.clone();
        (&mut result_data[4..8]).write_u32::<LittleEndian>(final_size)?;

        Ok(result_data)
    }
}

fn write_chunk<W: Write>(writer: &mut W, id: &[u8], data: &[u8]) -> R<()> {
    writer.write_all(id)?;
    writer.write_u32::<LittleEndian>(data.len() as u32)?;
    writer.write_all(data)?;
    if data.len() % 2 == 1 {
        writer.write_all(&[0])?; // padding
    }
    Ok(())
}

impl FlacCodec {
    pub fn extract_metadata_from_file(&self, file_path: &str) -> R<Metadata> {
        let tag = Tag::read_from_path(file_path)?;
        Ok(Metadata::Flac(tag))
    }
    pub fn embed_metadata_to_file(&self, file_path: &str, metadata: &Metadata) -> R<()> {
        let source_tag = match metadata {
            Metadata::Flac(tag) => tag,
            _ => return Err(anyhow!("Unsupported metadata format")),
        };
        let mut dest_tags = Tag::read_from_path(file_path)?;
        for block in source_tag.blocks() {
            match block {
                Block::VorbisComment(_)
                | Block::Picture(_)
                | Block::CueSheet(_)
                | Block::Application(_)
                | Block::Unknown(_) => {
                    dest_tags.push_block(block.clone());
                }
                _ => {}
            }
        }

        dest_tags.save()?;

        Ok(())
    }
}
