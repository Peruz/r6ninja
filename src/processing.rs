use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    // instrument
    pub freq:   String,
    pub height: String,

    // gps
    pub use_gps: bool,

    // range filter
    pub range_filter_enabled: bool,
    pub range_per_channel:    bool,
    pub range_global_min:     f64,
    pub range_global_max:     f64,
    pub range_min:            [f64; 6],
    pub range_max:            [f64; 6],
    pub range_action:         i32,   // 0=flag 1=remove

    // std filter
    pub std_filter_enabled: bool,
    pub std_threshold:      f64,
    pub std_action:         i32,   // 0=flag 1=remove
    pub std_window_mode:    i32,   // 0=time 1=samples
    pub std_window_seconds: f64,
    pub std_window_samples: i32,

    // smooth / downsample
    pub smooth_enabled:     bool,
    pub downsample_enabled: bool,
    pub agg_mode:           i32,   // 0=time 1=samples
    pub agg_samples:        i32,
    pub agg_seconds:        f64,
    pub agg_stat:           i32,   // 0=mean 1=median

    // output
    pub out_csv:    bool,
    pub out_duckdb: bool,
    pub out_emagpy: bool,

    // files
    pub files:       Vec<PathBuf>,
    pub merge_files: bool,
    pub table:       String,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            freq:   "9000".into(),
            height: "0.1".into(),
            use_gps: true,
            range_filter_enabled: false,
            range_per_channel:    false,
            range_global_min:    0.0,
            range_global_max:    100.0,
            range_min: [0.0; 6],
            range_max: [100.0; 6],
            range_action: 0,
            std_filter_enabled: true,
            std_threshold:      2.5,
            std_action:         0,
            std_window_mode:    0,
            std_window_seconds: 6.0,
            std_window_samples: 10,
            smooth_enabled:     false,
            downsample_enabled: false,
            agg_mode:           0,
            agg_samples:        5,
            agg_seconds:        1.0,
            agg_stat:           0,
            out_csv:    true,
            out_duckdb: false,
            out_emagpy: true,
            files:       vec![],
            merge_files: false,
            table:       "data".into(),
        }
    }
}
