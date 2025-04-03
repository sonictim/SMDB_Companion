use crate::prelude::*;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec::packet::traits::Mut;

pub struct AudioFile {
    // Audio data
    pub raw_pcm: Vec<u8>,
    pub sample_rate: u32,
    pub bit_depth: usize,
    pub channels: u16,
    pub format: ffmpeg::format::Sample,
    pub channel_layout: ffmpeg::channel_layout::ChannelLayout,

    // Metadata
    pub metadata: HashMap<String, String>,

    // FFmpeg context holders
    pub codec_id: ffmpeg::codec::Id,
    pub time_base: ffmpeg::util::rational::Rational,
    pub duration: i64,
    pub bit_rate: usize,
}

impl Default for AudioFile {
    fn default() -> Self {
        AudioFile {
            raw_pcm: Vec::new(),
            sample_rate: 0,
            bit_depth: 0,
            channels: 0,
            format: ffmpeg::format::Sample::U8(ffmpeg::format::sample::Type::Packed),
            channel_layout: ffmpeg::channel_layout::ChannelLayout::default(2),
            metadata: HashMap::new(),
            codec_id: ffmpeg::codec::Id::None,
            time_base: ffmpeg::util::rational::Rational::new(1, 1),
            duration: 0,
            bit_rate: 0,
        }
    }
}

impl AudioFile {
    /// Create a new empty AudioFile instance
    pub fn new(path: &Path) -> Self {
        let mut audio = AudioFile::default();
        let _ = audio.decode_from_file(path);
        audio
    }

    /// Initialize FFmpeg with debug information
    pub fn init_ffmpeg() -> Result<(), ffmpeg::Error> {
        println!("==== FFmpeg Initialization Debug Info ====");
        println!(
            "Current working directory: {:?}",
            std::env::current_dir().unwrap_or_default()
        );

        // Print relevant environment variables for library paths based on OS
        #[cfg(target_os = "linux")]
        println!(
            "LD_LIBRARY_PATH: {:?}",
            std::env::var("LD_LIBRARY_PATH").unwrap_or_default()
        );

        #[cfg(target_os = "macos")]
        println!(
            "DYLD_LIBRARY_PATH: {:?}",
            std::env::var("DYLD_LIBRARY_PATH").unwrap_or_default()
        );
        println!(
            "DYLD_FALLBACK_LIBRARY_PATH: {:?}",
            std::env::var("DYLD_FALLBACK_LIBRARY_PATH").unwrap_or_default()
        );

        #[cfg(target_os = "windows")]
        println!("PATH: {:?}", std::env::var("PATH").unwrap_or_default());

        println!("Attempting to initialize FFmpeg...");
        match ffmpeg::init() {
            Ok(_) => {
                println!("✓ FFmpeg initialized successfully!");

                // Try to print FFmpeg configuration info
                unsafe {
                    if let Ok(config) =
                        std::ffi::CStr::from_ptr(ffmpeg::sys::avcodec_configuration()).to_str()
                    {
                        println!("FFmpeg configuration: {}", config);
                    }
                }

                // Print available filters and codecs
                println!("FFmpeg version: {}", ffmpeg::util::version());
                println!("FFmpeg license: {}", ffmpeg::util::license());

                println!("==== FFmpeg Initialization Complete ====");
                Ok(())
            }
            Err(e) => {
                eprintln!("✗ Failed to initialize FFmpeg: {}", e);
                eprintln!("==== FFmpeg Initialization Failed ====");
                Err(e)
            }
        }
    }

    /// Decode an audio file into the struct
    pub fn decode_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), ffmpeg::Error> {
        // Open input file
        let mut input = ffmpeg::format::input(&path)?;

        // Find the first audio stream
        let stream = input
            .streams()
            .best(ffmpeg::media::Type::Audio)
            .ok_or(ffmpeg::Error::StreamNotFound)?;

        let stream_index = stream.index();

        // Get the decoder
        let context_decoder =
            ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
        let mut decoder = context_decoder.decoder().audio()?;

        // Copy stream info
        self.sample_rate = decoder.rate();
        self.channels = decoder.channels();
        self.bit_depth = decoder.format().bytes() * 8;
        self.format = decoder.format();
        self.channel_layout = decoder.channel_layout();
        self.codec_id = decoder.id();
        self.time_base = stream.time_base();
        self.duration = stream.duration();
        self.bit_rate = decoder.bit_rate();

        // Extract metadata from file
        self.extract_metadata(&input);

        // Read packets and decode frames
        let mut decoded_data: Vec<u8> = Vec::new();

        for (stream, packet) in input.packets() {
            if stream.index() == stream_index {
                let mut frame = ffmpeg::frame::Audio::empty();
                // Use receive_frame instead of decode
                match decoder.send_packet(&packet) {
                    Ok(_) => {}
                    Err(error) => eprintln!("Error sending packet: {}", error),
                }

                while decoder.receive_frame(&mut frame).is_ok() {
                    let data_size =
                        frame.samples() * frame.channels() as usize * frame.format().bytes();
                    let data_slice =
                        unsafe { std::slice::from_raw_parts(frame.data(0).as_ptr(), data_size) };
                    decoded_data.extend_from_slice(data_slice);
                }
            }
        }

        // Flush the decoder
        decoder.send_eof().ok();
        let mut frame = ffmpeg::frame::Audio::empty();
        while decoder.receive_frame(&mut frame).is_ok() {
            unsafe {
                if !frame.is_empty() {
                    let data_size =
                        frame.samples() * frame.channels() as usize * frame.format().bytes();
                    let data_slice = std::slice::from_raw_parts(frame.data(0).as_ptr(), data_size);
                    decoded_data.extend_from_slice(data_slice);
                }
            }
        }

        self.raw_pcm = decoded_data;

        Ok(())
    }

    /// Extract metadata from an input context
    fn extract_metadata(&mut self, input: &ffmpeg::format::context::Input) {
        // Clear existing metadata
        self.metadata.clear();

        // Get metadata from the input file
        for (k, v) in input.metadata().iter() {
            self.metadata.insert(k.into(), v.into());
        }

        // Get metadata from the audio stream if available
        if let Some(stream) = input.streams().best(ffmpeg::media::Type::Audio) {
            for (k, v) in stream.metadata().iter() {
                self.metadata.insert(format!("stream_{}", k), v.into());
            }
        }
    }

    /// Encode audio data to a file
    pub fn encode_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ffmpeg::Error> {
        // Create output context
        let mut output = ffmpeg::format::output(&path)?;

        // Find the encoder for the codec
        let codec = ffmpeg::encoder::find(self.codec_id).ok_or(ffmpeg::Error::EncoderNotFound)?;

        // Create a stream in the output
        let stream = output.add_stream(codec)?;
        let stream_index = stream.index();

        // Get the encoder context for configuration
        let context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;

        // Configure the encoder parameters
        {
            let mut encoder_context = context.encoder().audio()?;
            encoder_context.set_rate(self.sample_rate as i32);
            encoder_context.set_format(self.format);
            encoder_context.set_channel_layout(self.channel_layout);
            // Note: channels are derived from the channel layout, no need to set separately
            encoder_context.set_bit_rate(self.bit_rate);

            // Open the encoder
            let encoder = encoder_context.open_as(codec)?;

            // Set the stream's time_base from the encoder
            // Note: parameters are already properly set when we opened the encoder
            let mut output_stream = output.stream_mut(stream_index).expect("Stream not found");
            output_stream.set_time_base(encoder.time_base());
        }

        // Write format header
        output.write_header()?;

        // Get the encoder again after all configuration
        let context = ffmpeg::codec::context::Context::from_parameters(
            output
                .stream(stream_index)
                .expect("Stream not found")
                .parameters(),
        )?;
        let mut encoder = context.encoder().audio()?;

        // Create an audio frame to hold the data
        let mut frame = ffmpeg::frame::Audio::new(
            encoder.format(),
            1024, // Number of samples per frame (typical audio frame size)
            encoder.channel_layout(),
        );

        // Calculate frame size in bytes
        let frame_size = frame.samples() * self.channels as usize * self.format.bytes();

        // Process raw PCM data in chunks
        for chunk in self.raw_pcm.chunks(frame_size) {
            // If the chunk isn't a full frame, skip (this is simplified)
            if chunk.len() < frame_size {
                continue;
            }

            // Fill the frame with data
            unsafe {
                std::ptr::copy_nonoverlapping(
                    chunk.as_ptr(),
                    frame.data_mut(0).as_mut_ptr(),
                    frame_size,
                );
            }

            // Send the frame to the encoder
            encoder.send_frame(&frame)?;

            // Get the encoded packets
            self.receive_and_write_packets(&mut encoder, &mut output, stream_index)?;
        }

        // Flush the encoder
        encoder.send_eof()?;
        self.receive_and_write_packets(&mut encoder, &mut output, stream_index)?;

        // Write the trailer
        output.write_trailer()?;

        Ok(())
    }

    /// Helper function to receive packets from encoder and write them to output
    fn receive_and_write_packets(
        &self,
        encoder: &mut ffmpeg::encoder::audio::Audio,
        output: &mut ffmpeg::format::context::Output,
        stream_index: usize,
    ) -> Result<(), ffmpeg::Error> {
        let mut encoded = ffmpeg::Packet::empty();

        // Get the time base before any mutable operations on output
        let time_base = match output.stream(stream_index) {
            Some(s) => s.time_base(),
            None => return Err(ffmpeg::Error::StreamNotFound),
        };

        while encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(stream_index);
            encoded.rescale_ts(
                ffmpeg::util::rational::Rational::new(1, self.sample_rate as i32),
                time_base,
            );

            // Different approach - use a method from the Context trait
            unsafe {
                let result = ffmpeg::sys::av_interleaved_write_frame(
                    output.as_mut_ptr(),
                    encoded.as_mut_ptr(),
                );
                if result < 0 {
                    return Err(ffmpeg::Error::from(result));
                }
            }
        }

        Ok(())
    }

    /// Apply metadata to an output file using the ffmpeg API instead of CLI
    pub fn apply_metadata<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path_str = path.as_ref().to_str().unwrap_or("");
        let temp_path = format!("{}.temp", path_str);

        // Open input file
        let mut input = ffmpeg::format::input(&path)?;

        // Create output context for temp file
        let mut output = ffmpeg::format::output(&temp_path)?;

        // Copy streams and parameters
        for stream in input.streams() {
            let codec_id = stream.parameters().id();
            let codec = ffmpeg::encoder::find(codec_id).ok_or(ffmpeg::Error::EncoderNotFound)?;

            let mut out_stream = output.add_stream(codec)?;

            // Copy stream parameters
            unsafe {
                ffmpeg::sys::avcodec_parameters_copy(
                    (*out_stream.as_mut_ptr()).codecpar,
                    (*stream.as_ptr()).codecpar,
                );
            }

            // Set time base
            out_stream.set_time_base(stream.time_base());
        }

        // Create a new dictionary for output metadata
        let mut dict = ffmpeg::Dictionary::new();

        // Set metadata on the output context
        for (key, value) in &self.metadata {
            if !key.starts_with("stream_") {
                dict.set(key, value);
            }
        }

        // Apply the dictionary to the output context
        unsafe {
            let out_ptr = output.as_mut_ptr();
            if !(*out_ptr).metadata.is_null() {
                ffmpeg::sys::av_dict_free(&mut (*out_ptr).metadata);
            }
            // Disown directly assigns ownership of the dictionary to the metadata field
            (*out_ptr).metadata = dict.disown();
        }

        // Set metadata on the audio stream
        if let Some(stream) = output.streams().best(ffmpeg::media::Type::Audio) {
            let mut dict = ffmpeg::Dictionary::new();

            for (key, value) in &self.metadata {
                if key.starts_with("stream_") {
                    let clean_key = key.strip_prefix("stream_").unwrap_or(key);
                    dict.set(clean_key, value);
                }
            }

            unsafe {
                // Apply metadata to stream
                // Cast to mutable pointer since we need to modify the metadata
                let stream_ptr_mut = stream.as_ptr() as *mut ffmpeg::sys::AVStream;
                if !(*stream_ptr_mut).metadata.is_null() {
                    ffmpeg::sys::av_dict_free(&mut (*stream_ptr_mut).metadata);
                }

                // Directly assign the disowned dictionary to the metadata field
                (*stream_ptr_mut).metadata = dict.disown();
            }
        }

        // Write header
        output.write_header()?;

        // Copy packets
        for (stream, packet) in input.packets() {
            let mut pkt = packet;
            let stream_index = stream.index();

            // Set stream index for output
            pkt.set_stream(stream_index);

            // Write packet
            unsafe {
                let result =
                    ffmpeg::sys::av_interleaved_write_frame(output.as_mut_ptr(), pkt.as_mut_ptr());
                if result < 0 {
                    return Err(ffmpeg::Error::from(result).into());
                }
            }
        }

        // Write trailer
        output.write_trailer()?;

        // Close resources
        drop(input);
        drop(output);

        // Replace original with temp file
        std::fs::rename(temp_path, path)?;

        Ok(())
    }
    /// Combines encode_to_file and apply_metadata in one step
    pub fn encode_with_metadata<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.encode_to_file(&path)?;
        self.apply_metadata(&path)?;
        Ok(())
    }

    /// Set a metadata key-value pair
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Get a metadata value for a key
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Print a summary of the audio information
    pub fn print_summary(&self) {
        println!("Audio File Summary:");
        println!("------------------");
        println!("Sample Rate: {} Hz", self.sample_rate);
        println!("Channels: {}", self.channels);
        println!("Bit Depth: {} bits", self.bit_depth);
        println!(
            "Duration: {} seconds",
            self.duration as f64 * f64::from(self.time_base)
        );
        println!("Bit Rate: {} kbps", self.bit_rate / 1000);
        println!("Codec: {:?}", self.codec_id);
        println!("\nMetadata:");
        for (key, value) in &self.metadata {
            println!("  {}: {}", key, value);
        }
    }

    /// Extract a specific channel from multichannel audio
    pub fn extract_channel(&self, channel_index: usize) -> Result<AudioFile, &'static str> {
        // Check if the requested channel exists
        if channel_index >= self.channels as usize {
            return Err("Channel index out of bounds");
        }

        println!(
            "Extracting channel {} from multichannel audio ({} channels)",
            channel_index, self.channels
        );

        // Calculate bytes per sample
        let bytes_per_sample = self.format.bytes();
        let total_channels = self.channels as usize;

        // Create a new buffer for the mono channel
        let mut mono_pcm = Vec::with_capacity(self.raw_pcm.len() / total_channels);

        // Extract just the specified channel's samples from the interleaved PCM data
        for chunk in self.raw_pcm.chunks_exact(bytes_per_sample * total_channels) {
            // Calculate the start position of the desired channel in this frame
            let start = channel_index * bytes_per_sample;
            let end = start + bytes_per_sample;

            // Only copy bytes from the specified channel
            if start < chunk.len() && end <= chunk.len() {
                mono_pcm.extend_from_slice(&chunk[start..end]);
            }
        }

        println!(
            "Original PCM size: {} bytes, extracted channel PCM size: {} bytes",
            self.raw_pcm.len(),
            mono_pcm.len()
        );

        // Clone the metadata and add extraction info
        let mut metadata = self.metadata.clone();
        metadata.insert(
            "extracted_channel".to_string(),
            format!("{}", channel_index),
        );
        metadata.insert("source_channels".to_string(), format!("{}", self.channels));

        // Create a new AudioFile with all fields initialized at once
        let mono_audio = AudioFile {
            raw_pcm: mono_pcm,
            sample_rate: self.sample_rate,
            bit_depth: self.bit_depth,
            format: self.format,
            channels: 1,
            channel_layout: ffmpeg::channel_layout::ChannelLayout::default(1), // Mono layout
            codec_id: self.codec_id,
            time_base: self.time_base,
            duration: self.duration,
            bit_rate: self.bit_rate / total_channels, // Approximate bit rate adjustment
            metadata,
        };

        Ok(mono_audio)
    }

    /// Convenience method to extract just the first channel
    pub fn extract_first_channel(&self) -> Result<AudioFile, &'static str> {
        self.extract_channel(0)
    }

    /// Save a single channel to a file
    pub fn save_channel_to_file<P: AsRef<Path>>(
        &self,
        path: P,
        channel_index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Extract the channel
        let mono_audio = self.extract_channel(channel_index)?;

        // Save to file with metadata
        mono_audio.encode_with_metadata(path)?;

        println!("Successfully saved channel {} to file", channel_index);
        Ok(())
    }

    /// Convenience method to save the first channel to a file
    pub fn save_first_channel_to_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.save_channel_to_file(path, 0)
    }
}

pub fn cleanup_multi_mono(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    AudioFile::init_ffmpeg()?;

    // Check if it's already a mono file
    let audio = AudioFile::new(path);

    if audio.channels <= 1 {
        println!("File is already mono, skipping: {:?}", path);
        return Ok(());
    }

    println!(
        "Converting file from {} channels to mono: {:?}",
        audio.channels, path
    );

    // Create a temp path in the same directory
    let temp_path = path.with_extension(format!(
        "{}.temp.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        path.extension().unwrap_or_default().to_string_lossy()
    ));

    // Extract first channel to the temp file
    audio.save_first_channel_to_file(&temp_path)?;

    // Check the temp file exists and is valid
    if !temp_path.exists() || temp_path.metadata()?.len() == 0 {
        return Err("Failed to create mono file - output is empty".into());
    }

    // Replace the original with the temp file
    std::fs::rename(&temp_path, path)?;

    println!("Successfully converted to mono: {:?}", path);
    Ok(())
}
