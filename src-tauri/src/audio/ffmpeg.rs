use ffmpeg_next as ffmpeg;
use std::fs;
use std::path::{Path, PathBuf};

/// Main function to extract the first audio channel from a file
pub fn extract_first_audio_channel(input_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize FFmpeg
    ffmpeg::init()?;

    // Create paths
    let output_path = create_temp_output_path(input_path)?;

    // Open input and get stream info
    let (input_ctx, audio_stream, audio_stream_index, mut decoder) = open_input_file(input_path)?;

    // Get metadata and format info
    let (input_metadata, stream_metadata) = get_metadata(&input_ctx, &audio_stream);
    let format_name = input_ctx.format().name().to_string();

    // Setup output context with original codec
    let original_codec_id = audio_stream.codec().id();

    // Check if the codec is supported for encoding
    let codec = ffmpeg::encoder::find(original_codec_id);
    if codec.is_none() {
        return Err(format!(
            "Codec {:?} not found or not supported for encoding",
            original_codec_id
        )
        .into());
    }

    let mut output_ctx = setup_output_context(
        &output_path,
        &format_name,
        original_codec_id,
        &decoder,
        &audio_stream,
        &input_metadata,
        &stream_metadata,
    )?;

    // Process audio
    let result = process_audio(
        &input_ctx,
        audio_stream_index,
        &mut decoder,
        &mut output_ctx,
    );

    if let Err(e) = &result {
        // Clean up temporary file on error
        if output_path.exists() {
            let _ = fs::remove_file(&output_path);
        }
        return Err(format!("Error processing audio: {}", e).into());
    }

    // Replace the original file with the new one
    match fs::rename(&output_path, &input_path) {
        Ok(_) => Ok(()),
        Err(e) => {
            // Clean up temporary file on error
            let _ = fs::remove_file(&output_path);
            Err(format!("Failed to replace original file: {}", e).into())
        }
    }
}

/// Helper function to try different approaches if the primary one fails
pub fn extract_first_audio_channel_with_fallback(
    input_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match extract_first_audio_channel(input_path) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error with original codec, trying WAV fallback: {}", e);
            match extract_first_audio_channel_wav_fallback(input_path) {
                Ok(_) => Ok(()),
                Err(e) => {
                    // Try with MP3 as a second fallback if WAV fails
                    eprintln!("Error with WAV fallback, trying MP3: {}", e);
                    extract_first_audio_channel_mp3_fallback(input_path)
                }
            }
        }
    }
}

/// Create a temporary output path in the same directory
fn create_temp_output_path(input_path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut output_path = PathBuf::from(input_path);
    let file_stem = input_path.file_stem().ok_or("Invalid input filename")?;
    let extension = input_path.extension().ok_or("Invalid input extension")?;

    let temp_filename = format!(
        "{}_temp.{}",
        file_stem.to_string_lossy(),
        extension.to_string_lossy()
    );
    output_path.set_file_name(temp_filename);

    Ok(output_path)
}

/// Open the input file and get the audio stream
fn open_input_file(
    input_path: &Path,
) -> Result<
    (
        ffmpeg::format::context::Input,
        ffmpeg::format::stream::Stream,
        usize,
        ffmpeg::codec::decoder::Audio,
    ),
    Box<dyn std::error::Error>,
> {
    // Open the input file
    let input_ctx = ffmpeg::format::input(&input_path)?;

    // Find the audio stream
    let audio_stream = input_ctx
        .streams()
        .best(ffmpeg::media::Type::Audio)
        .ok_or("No audio stream found")?;

    let audio_stream_index = audio_stream.index();

    // Get the audio decoder
    let decoder_ctx = ffmpeg::codec::context::Context::from_parameters(audio_stream.parameters())?;
    let decoder = decoder_ctx.decoder().audio()?;

    Ok((input_ctx, audio_stream, audio_stream_index, decoder))
}

/// Get metadata from input context and audio stream
fn get_metadata(
    input_ctx: &ffmpeg::format::context::Input,
    audio_stream: &ffmpeg::format::stream::Stream,
) -> (ffmpeg::DictionaryRef, ffmpeg::DictionaryRef) {
    let input_metadata = input_ctx.metadata();
    let stream_metadata = audio_stream.metadata();

    (input_metadata, stream_metadata)
}

/// Setup the output context with appropriate codec
fn setup_output_context(
    output_path: &Path,
    format_name: &str,
    codec_id: ffmpeg::codec::Id,
    decoder: &ffmpeg::codec::decoder::Audio,
    audio_stream: &ffmpeg::format::stream::Stream,
    input_metadata: &ffmpeg::DictionaryRef,
    stream_metadata: &ffmpeg::DictionaryRef,
) -> Result<ffmpeg::format::context::Output, Box<dyn std::error::Error>> {
    // Prepare output format context with the same format
    let mut output_ctx = match ffmpeg::format::output_with_format(&output_path, &format_name) {
        Ok(ctx) => ctx,
        Err(e) => {
            return Err(format!(
                "Failed to create output context with format '{}': {}",
                format_name, e
            )
            .into());
        }
    };

    // Find the encoder for the original codec
    let codec = ffmpeg::encoder::find(codec_id)
        .ok_or(format!("Encoder for codec {:?} not found", codec_id))?;

    let mut output_stream = output_ctx.add_stream(codec)?;

    // Setup the encoder
    let mut encoder_ctx = match output_stream.codec().encoder().audio() {
        Ok(ctx) => ctx,
        Err(e) => return Err(format!("Failed to get encoder context: {}", e).into()),
    };

    configure_encoder(&mut encoder_ctx, decoder);

    // Copy time base from original
    if let Some(time_base) = audio_stream.time_base().as_rational() {
        output_stream.set_time_base(time_base);
    }

    // Open the encoder
    let _encoder = match encoder_ctx.open_as(codec) {
        Ok(enc) => enc,
        Err(e) => return Err(format!("Failed to open encoder: {}", e).into()),
    };

    // Copy metadata
    copy_metadata(
        &mut output_ctx,
        input_metadata,
        &mut output_stream,
        stream_metadata,
    );

    // Write the output header
    match output_ctx.write_header() {
        Ok(_) => Ok(output_ctx),
        Err(e) => Err(format!("Failed to write header: {}", e).into()),
    }
}

/// Configure the encoder with appropriate settings
fn configure_encoder(
    encoder_ctx: &mut ffmpeg::codec::context::Encoder,
    decoder: &ffmpeg::codec::decoder::Audio,
) {
    encoder_ctx.set_rate(decoder.rate() as i32);
    encoder_ctx.set_format(decoder.format());
    encoder_ctx.set_channel_layout(ffmpeg::channel_layout::ChannelLayout::MONO);
    encoder_ctx.set_channels(1);

    // Preserve the original bitrate if possible
    if decoder.bit_rate() > 0 {
        encoder_ctx.set_bit_rate(decoder.bit_rate());
    } else {
        // If bitrate is not available, set a reasonable default based on sample rate
        let default_bitrate = match decoder.rate() {
            r if r >= 48000 => 192000,
            r if r >= 44100 => 160000,
            r if r >= 32000 => 128000,
            r if r >= 22050 => 96000,
            _ => 64000,
        };
        encoder_ctx.set_bit_rate(default_bitrate);
    }
}

/// Copy metadata from input to output
fn copy_metadata(
    output_ctx: &mut ffmpeg::format::context::Output,
    input_metadata: &ffmpeg::DictionaryRef,
    output_stream: &mut ffmpeg::format::stream::StreamMut,
    stream_metadata: &ffmpeg::DictionaryRef,
) {
    // Copy global metadata
    for (key, value) in input_metadata.iter() {
        output_ctx.metadata().set(key, value);
    }

    // Copy stream metadata
    for (key, value) in stream_metadata.iter() {
        output_stream.metadata().set(key, value);
    }
}

/// Process audio from input to output
fn process_audio(
    input_ctx: &ffmpeg::format::context::Input,
    audio_stream_index: usize,
    decoder: &mut ffmpeg::codec::decoder::Audio,
    output_ctx: &mut ffmpeg::format::context::Output,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get the output stream and encoder
    let output_stream = output_ctx.stream(0).ok_or("No output stream")?;
    let mut encoder = match output_ctx.stream(0).codec().encoder().audio() {
        Ok(enc) => enc,
        Err(e) => return Err(format!("Failed to get encoder: {}", e).into()),
    };

    // Create resampler to handle channel conversion
    let mut resampler = match ffmpeg::software::resampling::context::Context::get(
        decoder.format(),
        decoder.channel_layout(),
        decoder.rate(),
        encoder.format(),
        encoder.channel_layout(),
        encoder.rate(),
    ) {
        Ok(ctx) => ctx,
        Err(e) => return Err(format!("Failed to create resampler: {}", e).into()),
    };

    // Allocate frame and packet
    let mut decoded = ffmpeg::frame::Audio::empty();
    let mut encoded = ffmpeg::Packet::empty();

    // Process the input file
    process_packets(
        input_ctx,
        audio_stream_index,
        decoder,
        &mut resampler,
        &mut encoder,
        &mut decoded,
        &mut encoded,
        output_ctx,
        &output_stream,
    )?;

    // Flush the decoder
    flush_decoder(
        decoder,
        &mut resampler,
        &mut encoder,
        &mut decoded,
        &mut encoded,
        output_ctx,
        &output_stream,
    )?;

    // Flush the encoder
    flush_encoder(&mut encoder, &mut encoded, output_ctx)?;

    // Write the trailer
    match output_ctx.write_trailer() {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to write trailer: {}", e).into()),
    }
}

/// Process all packets in the input file
fn process_packets(
    input_ctx: &ffmpeg::format::context::Input,
    audio_stream_index: usize,
    decoder: &mut ffmpeg::codec::decoder::Audio,
    resampler: &mut ffmpeg::software::resampling::context::Context,
    encoder: &mut ffmpeg::codec::encoder::Audio,
    decoded: &mut ffmpeg::frame::Audio,
    encoded: &mut ffmpeg::Packet,
    output_ctx: &mut ffmpeg::format::context::Output,
    output_stream: &ffmpeg::format::stream::Stream,
) -> Result<(), Box<dyn std::error::Error>> {
    for (stream, packet) in input_ctx.packets() {
        if stream.index() == audio_stream_index {
            decoder.send_packet(&packet)?;

            while decoder.receive_frame(decoded).is_ok() {
                // Resample to mono (first channel only)
                let mut resampled = ffmpeg::frame::Audio::empty();
                resampler.run(decoded, &mut resampled)?;

                // Send to encoder
                encoder.send_frame(&resampled)?;

                // Receive encoded packets
                while encoder.receive_packet(encoded).is_ok() {
                    // Ensure the packet is correctly associated with the output stream
                    encoded.set_stream(0);
                    encoded.rescale_ts(resampled.time_base(), output_stream.time_base());
                    output_ctx.write_packet(encoded)?;
                }
            }
        }
    }

    Ok(())
}

/// Flush the decoder
fn flush_decoder(
    decoder: &mut ffmpeg::codec::decoder::Audio,
    resampler: &mut ffmpeg::software::resampling::context::Context,
    encoder: &mut ffmpeg::codec::encoder::Audio,
    decoded: &mut ffmpeg::frame::Audio,
    encoded: &mut ffmpeg::Packet,
    output_ctx: &mut ffmpeg::format::context::Output,
    output_stream: &ffmpeg::format::stream::Stream,
) -> Result<(), Box<dyn std::error::Error>> {
    decoder.send_eof()?;
    while decoder.receive_frame(decoded).is_ok() {
        let mut resampled = ffmpeg::frame::Audio::empty();
        resampler.run(decoded, &mut resampled)?;
        encoder.send_frame(&resampled)?;

        while encoder.receive_packet(encoded).is_ok() {
            encoded.set_stream(0);
            encoded.rescale_ts(resampled.time_base(), output_stream.time_base());
            output_ctx.write_packet(encoded)?;
        }
    }

    Ok(())
}

/// Flush the encoder
fn flush_encoder(
    encoder: &mut ffmpeg::codec::encoder::Audio,
    encoded: &mut ffmpeg::Packet,
    output_ctx: &mut ffmpeg::format::context::Output,
) -> Result<(), Box<dyn std::error::Error>> {
    encoder.send_eof()?;
    while encoder.receive_packet(encoded).is_ok() {
        encoded.set_stream(0);
        output_ctx.write_packet(encoded)?;
    }

    Ok(())
}

/// Fallback function that uses WAV format
fn extract_first_audio_channel_wav_fallback(
    input_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize FFmpeg
    ffmpeg::init()?;

    // Create a temporary output path with .wav extension
    let mut output_path = PathBuf::from(input_path);
    let file_stem = input_path.file_stem().ok_or("Invalid input filename")?;

    let temp_filename = format!("{}_temp.wav", file_stem.to_string_lossy());
    output_path.set_file_name(temp_filename);

    // Open input and get stream info
    let (input_ctx, audio_stream, audio_stream_index, mut decoder) = open_input_file(input_path)?;

    // Get metadata
    let (input_metadata, stream_metadata) = get_metadata(&input_ctx, &audio_stream);

    // Setup WAV output context
    let mut output_ctx = ffmpeg::format::output_with_format(&output_path, "wav")?;

    // Find the PCM encoder
    let codec =
        ffmpeg::encoder::find(ffmpeg::codec::Id::PCM_S16LE).ok_or("PCM encoder not found")?;

    let mut output_stream = output_ctx.add_stream(codec)?;

    // Setup the encoder for WAV
    let mut encoder_ctx = output_stream.codec().encoder().audio()?;
    encoder_ctx.set_rate(decoder.rate() as i32);
    encoder_ctx.set_format(ffmpeg::format::Sample::I16(
        ffmpeg::format::sample::Type::Packed,
    ));
    encoder_ctx.set_channel_layout(ffmpeg::channel_layout::ChannelLayout::MONO);
    encoder_ctx.set_channels(1);

    // Open the encoder
    let _encoder = encoder_ctx.open_as(codec)?;

    // Copy metadata
    copy_metadata(
        &mut output_ctx,
        &input_metadata,
        &mut output_stream,
        &stream_metadata,
    );

    // Write the output header
    output_ctx.write_header()?;

    // Process audio
    process_audio(
        &input_ctx,
        audio_stream_index,
        &mut decoder,
        &mut output_ctx,
    )?;

    // Replace the original file with the new one
    fs::rename(&output_path, &input_path)?;

    Ok(())
}

/// MP3 fallback function for when WAV fails
fn extract_first_audio_channel_mp3_fallback(
    input_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize FFmpeg
    ffmpeg::init()?;

    // Create a temporary output path with .mp3 extension
    let mut output_path = PathBuf::from(input_path);
    let file_stem = input_path.file_stem().ok_or("Invalid input filename")?;

    let temp_filename = format!("{}_temp.mp3", file_stem.to_string_lossy());
    output_path.set_file_name(temp_filename);

    // Open input and get stream info
    let (input_ctx, audio_stream, audio_stream_index, mut decoder) = open_input_file(input_path)?;

    // Get metadata
    let (input_metadata, stream_metadata) = get_metadata(&input_ctx, &audio_stream);

    // Setup MP3 output context
    let mut output_ctx = ffmpeg::format::output_with_format(&output_path, "mp3")?;

    // Find the MP3 encoder
    let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::MP3).ok_or("MP3 encoder not found")?;

    let mut output_stream = output_ctx.add_stream(codec)?;

    // Setup the encoder for MP3
    let mut encoder_ctx = output_stream.codec().encoder().audio()?;
    encoder_ctx.set_rate(decoder.rate() as i32);
    encoder_ctx.set_channel_layout(ffmpeg::channel_layout::ChannelLayout::MONO);
    encoder_ctx.set_channels(1);
    encoder_ctx.set_bit_rate(128000); // 128kbps is a good default for MP3

    // Open the encoder
    let _encoder = encoder_ctx.open_as(codec)?;

    // Copy metadata
    copy_metadata(
        &mut output_ctx,
        &input_metadata,
        &mut output_stream,
        &stream_metadata,
    );

    // Write the output header
    output_ctx.write_header()?;

    // Process audio
    process_audio(
        &input_ctx,
        audio_stream_index,
        &mut decoder,
        &mut output_ctx,
    )?;

    // Replace the original file with the new one
    fs::rename(&output_path, &input_path)?;

    Ok(())
}
