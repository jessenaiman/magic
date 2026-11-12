<<<<<<< HEAD
//! Read-only waveform envelopes from craft-audio's media decoder.
use crate::{EngineError, Result, Session};
use serde_json::{Value, json};

pub fn read(s: &mut Session, p: &Value) -> Result<Value> {
    let bad = |message: &str| EngineError::InvalidParams(message.into());
    let path = s.song_audio.as_ref().ok_or_else(|| bad("Attach the song audio first"))?;
    let ext = path.extension().and_then(|e| e.to_str());
    let file = std::fs::File::open(path).map_err(|error| EngineError::InvalidParams(format!("Cannot read song audio: {error}")))?;
    let mut dec = craft_audio::Decoder::open(file, ext, &craft_audio::DecodeOptions::default())
        .map_err(|error| EngineError::InvalidParams(format!("Cannot read song audio: {error}")))?;
    if dec.info().channels == 0 {
        return Err(bad("The song source has no audio"));
    }
    let sample_rate = dec.info().sample_rate.max(1);
    let duration = if dec.length_known() {
        dec.info().duration_secs()
    } else {
        let probe_file = std::fs::File::open(path).map_err(|error| EngineError::InvalidParams(format!("Cannot read song audio: {error}")))?;
        craft_audio::Decoder::open(probe_file, ext, &craft_audio::DecodeOptions::default())
            .and_then(craft_audio::Decoder::probe)
            .map_err(|error| EngineError::InvalidParams(format!("Cannot read song audio: {error}")))?
            .duration_secs()
    };
=======
//! Exact `O(log n)` waveform & RMS loudness envelopes powered by `craft_audio::peaks::Peaks`.

use craft_audio::peaks::Peaks;
use craft_audio::{DecodeOptions, decode_with};
use serde_json::{Value, json};

use crate::{EngineError, Result, Session};

fn amp_to_dbfs(amp: f32) -> f64 {
    let clamped = f64::from(amp.abs()).max(1e-6);
    (20.0 * clamped.log10()).clamp(-96.0, 6.0)
}

pub fn read(s: &mut Session, p: &Value) -> Result<Value> {
    let bad = |message: &str| EngineError::InvalidParams(message.into());
    let raw_song = s.song_audio.as_ref().ok_or_else(|| bad("Attach the song audio first"))?;
    let path = if raw_song.is_file() {
        raw_song.clone()
    } else if let Ok(dir) = crate::workshop::project_dir(s) {
        dir.join(raw_song)
    } else {
        raw_song.clone()
    };
    let bytes = std::fs::read(&path).map_err(|error| EngineError::InvalidParams(format!("Cannot read song audio: {error}")))?;
    let ext = path.extension().and_then(|e| e.to_str());
    let opts = DecodeOptions::default().with_max_samples(craft_audio::DEFAULT_MAX_SAMPLES / 4);
    let (info, buf) = decode_with(&bytes, ext, &opts).map_err(|error| EngineError::InvalidParams(format!("Cannot decode song audio: {error}")))?;
    if buf.channels.is_empty() || info.sample_rate == 0 {
        return Err(bad("The song source has no audio channels"));
    }

    let sample_rate = info.sample_rate.max(1);
    let total_frames = buf.frames();
    let duration = (total_frames as f64) / f64::from(sample_rate);
    if duration <= 0.0 {
        return Err(bad("The song source has zero duration"));
    }

>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    let number = |key: &str, default: f64| -> Result<f64> {
        p.get(key)
            .map_or(Ok(default), |value| value.as_f64().filter(|v| v.is_finite()).ok_or_else(|| EngineError::InvalidParams(format!("{key} must be finite"))))
    };
    let start = number("start", 0.0)?;
    let requested_end = number("end", duration)?;
<<<<<<< HEAD
    if start < 0.0 || start >= duration || requested_end <= start || requested_end - start > 600.0 {
        return Err(bad("Choose an audio range of at most 600 seconds within the song"));
=======
    if start < 0.0 || start >= duration || requested_end <= start || requested_end - start > 1800.0 {
        return Err(bad("Choose an audio range of at most 1800 seconds within the song"));
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
    }
    let end = requested_end.min(duration);
    let bins = match p.get("bins") {
        Some(value) => value.as_u64().filter(|value| (8..=4096).contains(value)).ok_or_else(|| bad("bins must be an integer from 8 to 4096"))? as usize,
<<<<<<< HEAD
        None => 800,
    };
    let first = (start * f64::from(sample_rate)).floor().max(0.0) as u64;
    let frames = ((end - start) * f64::from(sample_rate)).ceil().max(1.0) as usize;
    let _ = dec.seek(first);
    let mut peaks = vec![[0.0_f32, 0.0_f32]; bins];
    let mut offset = 0usize;
    while offset < frames {
        let Some(block) = dec.next_block().map_err(|error| EngineError::InvalidParams(format!("Cannot decode song audio: {error}")))? else {
            break;
        };
        let count = block.frames().min(frames - offset);
        for channel in &block.channels {
            for (index, &sample) in channel.iter().take(count).enumerate() {
                if sample.is_finite() {
                    let bin = ((offset + index) * bins / frames).min(bins - 1);
                    peaks[bin][0] = peaks[bin][0].min(sample);
                    peaks[bin][1] = peaks[bin][1].max(sample);
                }
            }
        }
        offset += count;
    }
    let metadata = std::fs::metadata(path)?;
    Ok(json!({"source":path,"start":start,"end":end,"duration":duration,"bins":bins,
        "peaks":peaks,"decoder":"FilmCraft","source_bytes":metadata.len(),
        "modified_ms":metadata.modified().ok().and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok()).map(|time| time.as_millis())}))
=======
        None => 360,
    };

    let mut mono = vec![0.0_f32; total_frames];
    let inv_ch = 1.0_f32 / (buf.channels.len() as f32);
    let mut max_abs = 0.0_f32;
    for ch in &buf.channels {
        for (i, &s_val) in ch.iter().take(total_frames).enumerate() {
            if s_val.is_finite() {
                mono[i] += s_val * inv_ch;
                max_abs = max_abs.max(s_val.abs());
            }
        }
    }

    let pyramid = Peaks::build(&mono);
    let start_sample = (start * f64::from(sample_rate)).max(0.0);
    let span_samples = ((end - start) * f64::from(sample_rate)).max(1.0);
    let samples_per_col = (span_samples / (bins as f64)).max(1.0);

    let min_max_cols = pyramid.spans(&mono, start_sample, samples_per_col, 0.0, bins);
    let rms_cols = pyramid.rms_spans(&mono, start_sample, samples_per_col, 0.0, bins);

    let peaks: Vec<[f32; 2]> = min_max_cols.into_iter().map(|opt| opt.map_or([0.0_f32, 0.0_f32], |(mn, mx)| [mn, mx])).collect();
    let rms: Vec<f32> = rms_cols.into_iter().map(|opt| opt.unwrap_or(0.0_f32)).collect();
    let mean_sq = if rms.is_empty() { 0.0_f32 } else { rms.iter().map(|r| r * r).sum::<f32>() / (rms.len() as f32) };
    let integrated_rms_dbfs = amp_to_dbfs(mean_sq.sqrt());
    let true_peak_dbfs = amp_to_dbfs(max_abs);

    let metadata = std::fs::metadata(&path)?;
    Ok(json!({
        "source": path,
        "start": start,
        "end": end,
        "duration": duration,
        "sample_rate": sample_rate,
        "channels": info.layout.channels(),
        "layout": info.layout.to_string(),
        "codec": info.codec.name(),
        "container": info.container.name(),
        "damaged": info.damaged,
        "bins": bins,
        "peaks": peaks,
        "rms": rms,
        "integrated_rms_dbfs": integrated_rms_dbfs,
        "true_peak_dbfs": true_peak_dbfs,
        "decoder": "craft-audio",
        "source_bytes": metadata.len(),
        "modified_ms": metadata.modified().ok().and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok()).map(|time| time.as_millis()),
    }))
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
}
