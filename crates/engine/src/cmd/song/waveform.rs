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

    let number = |key: &str, default: f64| -> Result<f64> {
        p.get(key)
            .map_or(Ok(default), |value| value.as_f64().filter(|v| v.is_finite()).ok_or_else(|| EngineError::InvalidParams(format!("{key} must be finite"))))
    };
    let start = number("start", 0.0)?;
    let requested_end = number("end", duration)?;
    if start < 0.0 || start >= duration || requested_end <= start || requested_end - start > 1800.0 {
        return Err(bad("Choose an audio range of at most 1800 seconds within the song"));
    }
    let end = requested_end.min(duration);
    let bins = match p.get("bins") {
        Some(value) => value.as_u64().filter(|value| (8..=4096).contains(value)).ok_or_else(|| bad("bins must be an integer from 8 to 4096"))? as usize,
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
}
