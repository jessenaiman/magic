#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug)]
pub struct StudioViewState {
    pub inspector_tab: String,
    pub selected_scene: usize,
    pub seek_seconds: f64,
    pub safe_margins: bool,
    pub snap_enabled: bool,
    pub monitor_fit: bool,
    pub show_export_settings: bool,
    pub export_format: String,
    pub export_container: String,
    pub export_hdr_transfer: String,
    pub bitrate_mode: String,
    pub crf_value: f64,
    pub hw_encoding: bool,
    pub export_scale: f64,
    pub export_quality: f64,
    pub burn_captions: bool,
    pub allow_offline: bool,
    pub gpu_rendering: bool,
    pub queue_export: bool,
    pub range_start: f64,
}

impl Default for StudioViewState {
    fn default() -> Self {
        Self {
            inspector_tab: "matrix".into(),
            selected_scene: 0,
            seek_seconds: 0.0,
            safe_margins: false,
            snap_enabled: true,
            monitor_fit: true,
            show_export_settings: false,
            export_format: "h264".into(),
            export_container: "mp4".into(),
            export_hdr_transfer: "sdr".into(),
            bitrate_mode: "crf".into(),
            crf_value: 18.0,
            hw_encoding: false,
            export_scale: 1.0,
            export_quality: 85.0,
            burn_captions: false,
            allow_offline: false,
            gpu_rendering: true,
            queue_export: false,
            range_start: 0.0,
        }
    }
}
