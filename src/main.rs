slint::include_modules!();

mod processing;
mod sql;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Result};
use duckdb::Connection;

use processing::ProcessingConfig;

// ── Warning helpers ───────────────────────────────────────────────────────────
fn error_log(ui: &slint::Weak<AppWindow>, msg: &str) {
    append_log(ui, &format!("[WARN!!!]  {msg}"));
}
fn info(ui: &slint::Weak<AppWindow>, msg: &str) {
    append_log(ui, &format!("[INFO]  {msg}"));
}

fn append_log(ui_handle: &slint::Weak<AppWindow>, msg: &str) {
    let msg = msg.to_string();
    let handle = ui_handle.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(ui) = handle.upgrade() {
            let current = ui.get_log_text().to_string();
            ui.set_log_text(format!("{current}\n{msg}").into());
        }
    }).ok();
}

// ── Collect config from UI ────────────────────────────────────────────────────
fn collect_config(ui: &AppWindow) -> ProcessingConfig {
    ProcessingConfig {
        freq:   ui.get_freq().to_string(),
        height: ui.get_sensor_height().to_string(),
        use_gps: ui.get_use_gps(),
        range_filter_enabled: ui.get_range_filter_enabled(),
        range_per_channel:    ui.get_range_per_channel(),
        range_global_min:     ui.get_range_global_min() as f64,
        range_global_max:     ui.get_range_global_max() as f64,
        range_min: [
            ui.get_range_min1() as f64, ui.get_range_min2() as f64,
            ui.get_range_min3() as f64, ui.get_range_min4() as f64,
            ui.get_range_min5() as f64, ui.get_range_min6() as f64,
        ],
        range_max: [
            ui.get_range_max1() as f64, ui.get_range_max2() as f64,
            ui.get_range_max3() as f64, ui.get_range_max4() as f64,
            ui.get_range_max5() as f64, ui.get_range_max6() as f64,
        ],
        range_action:         ui.get_range_action(),
        std_filter_enabled:   ui.get_std_filter_enabled(),
        std_threshold:        ui.get_std_threshold() as f64,
        std_action:           ui.get_std_action(),
        std_window_mode:      ui.get_std_window_mode(),
        std_window_seconds:   ui.get_std_window_seconds() as f64,
        std_window_samples:   ui.get_std_window_samples(),
        smooth_enabled:       ui.get_smooth_enabled(),
        downsample_enabled:   ui.get_downsample_enabled(),
        agg_mode:             ui.get_agg_mode(),
        agg_samples:          ui.get_agg_samples(),
        agg_seconds:          ui.get_agg_seconds() as f64,
        agg_stat:             ui.get_agg_stat(),
        out_csv:              ui.get_out_csv(),
        out_duckdb:           ui.get_out_duckdb(),
        out_emagpy:           ui.get_out_emagpy(),
        files:                vec![],
        merge_files:          ui.get_merge_files(),
        table:                "data".into(),
    }
}

// ── Validation ────────────────────────────────────────────────────────────────
fn run_validations(
    conn: &Connection,
    cfg: &ProcessingConfig,
    ui: &slint::Weak<AppWindow>,
) -> Result<()> {

    // row count
    let n: i64 = conn
        .query_row(sql::step_validate_row_count(), [], |r| r.get(0))
        .unwrap_or(0);
    info(ui, &format!("{n} $PGALA rows parsed"));
    if n < 50 {
        error_log(ui, &format!("Only {n} rows found (< 50 minimum)"));
    }

    // window vs dataset
    if cfg.std_filter_enabled && cfg.std_window_mode == 1 {
        if cfg.std_window_samples > n as i32 {
            error_log(ui, &format!(
                "Std window ({} samples) larger than dataset ({n} rows)",
                cfg.std_window_samples
            ));
        }
    }
    if (cfg.smooth_enabled || cfg.downsample_enabled) && cfg.agg_mode == 1 {
        if cfg.agg_samples > n as i32 {
            error_log(ui, &format!(
                "Aggregation window ({} samples) larger than dataset ({n} rows)",
                cfg.agg_samples
            ));
        }
    }

    // GNZDA check
    let nzda: i64 = conn
        .query_row(sql::step_validate_gnzda(), [], |r| r.get(0))
        .unwrap_or(0);
    if nzda == 0 {
        error_log(ui, "No $GNZDA rows found — date unavailable");
    }

    // GNGGA check
    let ngga: i64 = conn
        .query_row(sql::step_validate_gngga(), [], |r| r.get(0))
        .unwrap_or(0);
    if ngga == 0 {
        error_log(ui, "No $GNGGA rows found — GPS unavailable");
    }

    // GPS quality checks
    if cfg.use_gps && n > 0 {
        let mut stmt = conn.prepare(sql::step_validate_gps())?;
        let row = stmt.query_row([], |r| {
            Ok((
                r.get::<_, i64>(0)?,  // total
                r.get::<_, i64>(1)?,  // null_gps
                r.get::<_, i64>(2)?,  // low_numsv
                r.get::<_, i64>(3)?,  // high_hdop
                r.get::<_, i64>(4)?,  // high_pitch
                r.get::<_, i64>(5)?,  // high_roll
            ))
        })?;

        let (total, null_gps, low_numsv, high_hdop, high_pitch, high_roll) = row;
        let pct = |x: i64| (x as f64 / total as f64) * 100.0;

        if null_gps > 0 {
            let p = pct(null_gps);
            let sym = if p > 5.0 { "[WARN!!!]" } else { "[WARN]" };
            append_log(ui, &format!(
                "{sym}  {null_gps} rows ({p:.1}%) have null/invalid GPS coordinates"
            ));
        }
        if low_numsv > 0 {
            let p = pct(low_numsv);
            let sym = if p > 5.0 { "[WARN!!!]" } else { "[WARN]" };
            append_log(ui, &format!(
                "{sym}  {low_numsv} rows ({p:.1}%) have numsv < 8"
            ));
        }
        if high_hdop > 0 {
            let p = pct(high_hdop);
            let sym = if p > 5.0 { "[WARN!!!]" } else { "[WARN]" };
            append_log(ui, &format!(
                "{sym}  {high_hdop} rows ({p:.1}%) have hdop > 2.0"
            ));
        }
        if high_pitch > 0 {
            let p = pct(high_pitch);
            let sym = if p > 5.0 { "[WARN!!!]" } else { "[WARN]" };
            append_log(ui, &format!(
                "{sym}  {high_pitch} rows ({p:.1}%) have |pitch| > 5°"
            ));
        }
        if high_roll > 0 {
            let p = pct(high_roll);
            let sym = if p > 5.0 { "[WARN!!!]" } else { "[WARN]" };
            append_log(ui, &format!(
                "{sym}  {high_roll} rows ({p:.1}%) have |roll| > 5°"
            ));
        }
    }

    Ok(())
}

// ── Filter validation ─────────────────────────────────────────────────────────
fn run_filter_validation(
    conn: &Connection,
    ui: &slint::Weak<AppWindow>,
) -> Result<()> {
    let mut stmt = conn.prepare(sql::step_validate_filter())?;
    let (total, flagged): (i64, i64) = stmt.query_row([], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })?;

    if total > 0 && flagged > 0 {
        let pct = (flagged as f64 / total as f64) * 100.0;
        let sym = if pct > 20.0 { "[WARN!!!]" } else { "[WARN]" };
        append_log(ui, &format!(
            "{sym}  Filter flagged {flagged} rows ({pct:.1}%) \
             {}",
            if pct > 20.0 { "(> 20% threshold)" } else { "" }
        ));
    }
    info(ui, &format!(
        "{} rows remaining after filtering",
        total - flagged
    ));
    Ok(())
}

// ── Process a single file ─────────────────────────────────────────────────────
fn process_one(
    conn: &Connection,
    datafile: &PathBuf,
    table_name: &str,
    cfg: &ProcessingConfig,
    ui: &slint::Weak<AppWindow>,
) -> Result<()> {

    let outfile = datafile
        .parent().unwrap()
        .join(format!(
            "out_{}",
            datafile.file_name().unwrap_or_default().to_string_lossy()
        ))
        .with_extension("csv");

    info(ui, "Step 1/7: Parsing raw data...");
    conn.execute_batch(&sql::step_parse(
        &datafile.to_string_lossy()
    ))?;

    info(ui, "Step 2/7: Joining GPS and EM data...");
    conn.execute_batch(&sql::step_join(cfg.use_gps))?;

    info(ui, "Step 3/7 Check missing values...");
    conn.execute_batch(&sql::step_set_nan_to_null())?;
    let mut stmt = conn.prepare(&sql::step_check_nulls())?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
        ))
    })?;
    let mut missing_values = false;
    for row in rows {
        let (column_name, null_percentage) = row?;
        if null_percentage > 0.0 {
            append_log(ui, &format!("[WARN] {column_name} has {null_percentage} pct of missing data (nulls)."));
            missing_values = true;
        }
    }
    if missing_values == false {
        append_log(ui, "[OK] No missing values.");
    }

    info(ui, "Step 4/7: Validating data...");
    run_validations(conn, cfg, ui)?;

    // save SQL for debugging
    let sql_file = datafile.parent().unwrap().join("last_query.sql");
    let debug_sql = format!(
        "-- Step 1 parse data\n{}
        \n-- Step 2 join GPS and DATA rows\n{}
        \n-- Step 3 set missing to nulls\n{}
        \n-- Step 4 eca range filter\n{}
        \n --Step 5 eca std filter\n{}
        \n",
        sql::step_parse(&datafile.to_string_lossy()),
        sql::step_join(cfg.use_gps),
        sql::step_set_nan_to_null(),
        if cfg.range_filter_enabled {
            sql::step_range_filter(cfg)
        } else {
            "-- eca range filter disabled\n".to_string()
        },
        if cfg.std_filter_enabled {
            format!("{}\n{}\n",sql::step_std_stats(cfg), sql::step_std_filter(cfg))
        } else {
            "-- std filter disabled\n".to_string()
        },
    );
    std::fs::write(&sql_file, &debug_sql).ok();

    info(ui, "Step 5/7: Applying filters...");
    if cfg.range_filter_enabled {
        conn.execute_batch(&sql::step_range_filter(cfg))?;
    }

    if cfg.std_filter_enabled {
        conn.execute_batch(&sql::step_std_stats(cfg))?;
        conn.execute_batch(&sql::step_std_filter(cfg))?;
    }
    run_filter_validation(conn, ui)?;

    if cfg.smooth_enabled {
        info(ui, "Step 6/7: Smoothing...");
        conn.execute_batch(&sql::step_smooth(cfg))?;
    } else if cfg.downsample_enabled {
        info(ui, "Step 6/7: Downsampling...");
        conn.execute_batch(&sql::step_downsample(cfg))?;
    } else {
        info(ui, "Step 6/7: No aggregation.");
    }

    info(ui, "Step 7/7: Building output table...");
    conn.execute_batch(&sql::step_output(table_name, cfg))?;

    if cfg.out_csv {
        conn.execute_batch(&sql::copy_to_csv(
            table_name,
            &outfile.to_string_lossy(),
        ))?;
        append_log(ui, &format!("[OK] CSV: {}", outfile.display()));
    }

    Ok(())
}

// ── Process all files ─────────────────────────────────────────────────────────
fn process_files(
    files: Vec<PathBuf>,
    cfg: ProcessingConfig,
    ui_handle: slint::Weak<AppWindow>,
) -> Result<()> {

    let conn = if cfg.out_duckdb {
        let db_path = files[0].parent().unwrap()
            .join("r6ninja_output.duckdb");
        append_log(&ui_handle, &format!("[DB] DuckDB: {}", db_path.display()));
        Connection::open(db_path)?
    } else {
        Connection::open_in_memory()?
    };

    if cfg.merge_files {
        append_log(&ui_handle, "[MERGE] Merging all files...");

        let union_query: String = files
            .iter()
            .map(|f| format!(
                "select * from read_csv('{}', \
                 all_varchar=true, null_padding=true, \
                 sep=',', header=false)",
                f.display()
            ))
            .collect::<Vec<_>>()
            .join("\nunion all\n");

        let merged_path = files[0].parent().unwrap()
            .join("merged.csv");
        conn.execute_batch(&format!(
            "copy ({union_query}) to '{}' (header false, delimiter ',')",
            merged_path.display()
        ))?;

        let merged_pb = merged_path.clone();
        process_one(
            &conn,
            &merged_pb,
            &cfg.table,
            &cfg,
            &ui_handle,
        )?;

        std::fs::remove_file(&merged_path).ok();

    } else {
        for (i, file) in files.iter().enumerate() {
            let table_name = format!(
                "{}_{}",
                cfg.table,
                file.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
            );
            append_log(&ui_handle, &format!(
                "\n[PROC]  [{}/{}] Processing: {}",
                i + 1, files.len(),
                file.file_name().unwrap_or_default().to_string_lossy()
            ));
            process_one(&conn, file, &table_name, &cfg, &ui_handle)?;
        }
    }

    Ok(())
}

// ── Main ──────────────────────────────────────────────────────────────────────
fn main() -> Result<()> {
    unsafe {
        std::env::set_var("SLINT_BACKEND", "software");
    }

    let ui = AppWindow::new()?;
    let files: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(vec![]));

    // pick-files
    {
        let ui_handle = ui.as_weak();
        let files = files.clone();
        ui.on_pick_files(move || {
            if let Some(paths) = rfd::FileDialog::new()
                .add_filter("Raw data", &["txt", "csv", "dat", "raw"])
                .set_title("Select R6 data files")
                .pick_files()
            {
                let mut lock = files.lock().unwrap();
                for p in &paths {
                    if !lock.contains(p) {
                        lock.push(p.clone());
                    }
                }
                let display = lock
                    .iter()
                    .map(|p| p.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                if let Some(ui) = ui_handle.upgrade() {
                    ui.set_file_list_display(display.into());
                }
            }
        });
    }

    // clear-files
    {
        let ui_handle = ui.as_weak();
        let files = files.clone();
        ui.on_clear_files(move || {
            files.lock().unwrap().clear();
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_file_list_display("".into());
            }
        });
    }

    // run-processing
    {
        let ui_handle = ui.as_weak();
        let files = files.clone();
        ui.on_run_processing(move || {
            let file_list = files.lock().unwrap().clone();
            if file_list.is_empty() {
                if let Some(ui) = ui_handle.upgrade() {
                    ui.set_log_text("[WARN]  No files selected!".into());
                }
                return;
            }

            let mut cfg = if let Some(ui) = ui_handle.upgrade() {
                collect_config(&ui)
            } else {
                return;
            };
            cfg.files = file_list.clone();

            if let Some(ui) = ui_handle.upgrade() {
                ui.set_is_running(true);
                ui.set_log_text("".into());
            }

            let handle = ui_handle.clone();
            thread::spawn(move || {
                let result = process_files(file_list, cfg, handle.clone());
                slint::invoke_from_event_loop(move || {
                    if let Some(ui) = handle.upgrade() {
                        // set running false AFTER updating log
                        match result {
                            Ok(_) => {
                                let current = ui.get_log_text().to_string();
                                ui.set_log_text(
                                    format!("{current}\n[DONE] All done!").into()
                                );
                            }
                            Err(e) => {
                                let current = ui.get_log_text().to_string();
                                ui.set_log_text(
                                    format!("{current}\n[ERROR] {e}").into()
                                );
                            }
                        }
                        ui.set_is_running(false);
                    }
                }).ok();
            });
        });
    }

    ui.run()?;
    Ok(())
}
