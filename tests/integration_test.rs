use duckdb::Connection;
use r6ninja::sql;
use r6ninja::processing::ProcessingConfig;

// ── Test file constants ───────────────────────────────────────────────────────
const TEST1_ROWS: i64 = 22;
const TEST1_HIGH_PITCH: i64 = 5;
const TEST1_HIGH_PITCH_PCT: f64 = 5.0 / 22.0 * 100.0;
const TEST1_HIGH_ROLL: i64 = 3;
const TEST1_HIGH_ROLL_PCT: f64 = 3.0 / 22.0 * 100.0;

const TEST2_ROWS: i64 = 26;
const TEST2_DOWNSAMPLE_BUCKETS: i64 = (TEST2_ROWS + 4) / 5; // ceiling = 6

fn test_conn() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn base_cfg() -> ProcessingConfig {
    ProcessingConfig {
        use_gps: true,
        std_filter_enabled: false,
        range_filter_enabled: false,
        smooth_enabled: false,
        downsample_enabled: false,
        out_emagpy: true,
        out_csv: false,
        out_duckdb: false,
        ..ProcessingConfig::default()
    }
}

fn run_pipeline(conn: &Connection, datafile: &str, cfg: &ProcessingConfig) {
    conn.execute_batch(&sql::step_parse(datafile)).unwrap();
    conn.execute_batch(&sql::step_join(cfg.use_gps)).unwrap();
    if cfg.range_filter_enabled {
        conn.execute_batch(&&sql::step_range_filter(cfg)).unwrap();
    }
    if cfg.std_filter_enabled {
        conn.execute_batch(&sql::step_std_stats(cfg)).unwrap();
        conn.execute_batch(&&sql::step_std_filter(cfg)).unwrap();
    }
    if cfg.smooth_enabled {
        conn.execute_batch(&sql::step_smooth(cfg)).unwrap();
    } else if cfg.downsample_enabled {
        conn.execute_batch(&sql::step_downsample(cfg)).unwrap();
    }
    conn.execute_batch(&sql::step_output("result", cfg)).unwrap();
}

// ── Test 1: parse ─────────────────────────────────────────────────────────────
#[test]
fn test_parse_row_count() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    let n: i64 = conn
        .query_row("select count(*) from pgala", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, TEST1_ROWS,
        "Expected {TEST1_ROWS} $PGALA rows in test1.txt, got {n}");
}

#[test]
fn test_parse_gngga_count() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    let n: i64 = conn
        .query_row("select count(*) from gngga", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, TEST1_ROWS,
        "Expected {TEST1_ROWS} $GNGGA rows in test1.txt, got {n}");
}

#[test]
fn test_parse_gnzda_present() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    let n: i64 = conn
        .query_row("select count(*) from gnzda", [], |r| r.get(0))
        .unwrap();
    assert!(n > 0, "Expected at least one $GNZDA row");
}

// ── Test 2: GPS join ──────────────────────────────────────────────────────────
#[test]
fn test_gps_join_row_count() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    conn.execute_batch(&sql::step_join(true)).unwrap();
    let n: i64 = conn
        .query_row("select count(*) from data", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, TEST1_ROWS,
        "Expected {TEST1_ROWS} rows after GPS join, got {n}");
}

#[test]
fn test_gps_coordinates_constant() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    conn.execute_batch(&sql::step_join(true)).unwrap();
    let (lat, lon): (f64, f64) = conn
        .query_row(
            "select min(latitude), min(longitude) from data",
            [], |r| Ok((r.get(0)?, r.get(1)?))
        ).unwrap();
    assert!((lat - 45.0).abs() < 1e-6,
        "Expected latitude 45.0, got {lat}");
    assert!((lon - 12.0).abs() < 1e-6,
        "Expected longitude 12.0, got {lon}");
}

#[test]
fn test_no_gps_join() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    conn.execute_batch(&sql::step_join(false)).unwrap();
    let n: i64 = conn
        .query_row("select count(*) from data", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, TEST1_ROWS,
        "Expected {TEST1_ROWS} rows after no-GPS join, got {n}");
    let null_lat: i64 = conn
        .query_row(
            "select count(*) from data where latitude is null",
            [], |r| r.get(0)
        ).unwrap();
    assert_eq!(null_lat, TEST1_ROWS,
        "All latitude should be null when GPS disabled, got {null_lat}");
}

// ── Test 3: validation ────────────────────────────────────────────────────────
#[test]
fn test_validate_pitch_count() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    conn.execute_batch(&sql::step_join(true)).unwrap();
    let high_pitch: i64 = conn
        .query_row(
            "select count(*) from data where abs(pitch) > 5.0",
            [], |r| r.get(0)
        ).unwrap();
    assert_eq!(high_pitch, TEST1_HIGH_PITCH,
        "Expected {TEST1_HIGH_PITCH} rows with |pitch| > 5°, got {high_pitch}");
}

#[test]
fn test_validate_roll_count() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    conn.execute_batch(&sql::step_join(true)).unwrap();
    let high_roll: i64 = conn
        .query_row(
            "select count(*) from data where abs(roll) > 5.0",
            [], |r| r.get(0)
        ).unwrap();
    assert_eq!(high_roll, TEST1_HIGH_ROLL,
        "Expected {TEST1_HIGH_ROLL} rows with |roll| > 5°, got {high_roll}");
}

#[test]
fn test_validate_pitch_percentage() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    conn.execute_batch(&sql::step_join(true)).unwrap();
    let (total, high_pitch): (i64, i64) = conn
        .query_row(
            "select count(*), count(*) filter (where abs(pitch) > 5.0)
             from data",
            [], |r| Ok((r.get(0)?, r.get(1)?))
        ).unwrap();
    let pct = (high_pitch as f64 / total as f64) * 100.0;
    assert!((pct - TEST1_HIGH_PITCH_PCT).abs() < 0.1,
        "Expected {TEST1_HIGH_PITCH_PCT:.1}% high pitch, got {pct:.1}%");
}

#[test]
fn test_validate_roll_percentage() {
    let conn = test_conn();
    conn.execute_batch(&sql::step_parse("tests/data/test1.txt")).unwrap();
    conn.execute_batch(&sql::step_join(true)).unwrap();
    let (total, high_roll): (i64, i64) = conn
        .query_row(
            "select count(*), count(*) filter (where abs(roll) > 5.0)
             from data",
            [], |r| Ok((r.get(0)?, r.get(1)?))
        ).unwrap();
    let pct = (high_roll as f64 / total as f64) * 100.0;
    assert!((pct - TEST1_HIGH_ROLL_PCT).abs() < 0.1,
        "Expected {TEST1_HIGH_ROLL_PCT:.1}% high roll, got {pct:.1}%");
}

// ── Test 4: EMagPy columns ────────────────────────────────────────────────────
#[test]
fn test_emagpy_quad_all_ones() {
    let conn = test_conn();
    let cfg = base_cfg();
    run_pipeline(&conn, "tests/data/test1.txt", &cfg);

    let seps = ["0.5", "0.7", "0.9", "1.1", "1.5", "1.95"];
    for sep in &seps {
        let col = format!("\"HCP{sep}f9000h0.1_quad\"");
        let (min, max): (f64, f64) = conn
            .query_row(
                &format!("select min({col}), max({col}) from result"),
                [], |r| Ok((r.get(0)?, r.get(1)?))
            ).unwrap();
        assert!((min - 1.0).abs() < 1e-4,
            "HCP{sep}_quad min expected 1.0, got {min}");
        assert!((max - 1.0).abs() < 1e-4,
            "HCP{sep}_quad max expected 1.0, got {max}");
    }
}

// ── Test 5: constant values survive smoothing ─────────────────────────────────
#[test]
fn test_smooth_samples_constant() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        smooth_enabled: true,
        agg_mode: 1,
        agg_samples: 5,
        agg_stat: 0,
        std_filter_enabled: false,
        range_filter_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let expected = [10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0];
    for (i, exp) in expected.iter().enumerate() {
        let ch = i + 1;
        let val: f64 = conn
            .query_row(
                &format!("select avg(eca{ch}) from result"),
                [], |r| r.get(0)
            ).unwrap();
        assert!((val - exp).abs() < 1e-6,
            "Smooth samples: eca{ch} expected {exp}, got {val}");
    }
}

#[test]
fn test_smooth_time_constant() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        smooth_enabled: true,
        agg_mode: 0,
        agg_seconds: 1.0,
        agg_stat: 0,
        std_filter_enabled: false,
        range_filter_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let expected = [10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0];
    for (i, exp) in expected.iter().enumerate() {
        let ch = i + 1;
        let val: f64 = conn
            .query_row(
                &format!("select avg(eca{ch}) from result"),
                [], |r| r.get(0)
            ).unwrap();
        assert!((val - exp).abs() < 1e-6,
            "Smooth time: eca{ch} expected {exp}, got {val}");
    }
}

#[test]
fn test_smooth_median_constant() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        smooth_enabled: true,
        agg_mode: 1,
        agg_samples: 5,
        agg_stat: 1,
        std_filter_enabled: false,
        range_filter_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let expected = [10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0];
    for (i, exp) in expected.iter().enumerate() {
        let ch = i + 1;
        let val: f64 = conn
            .query_row(
                &format!("select avg(eca{ch}) from result"),
                [], |r| r.get(0)
            ).unwrap();
        assert!((val - exp).abs() < 1e-6,
            "Smooth median: eca{ch} expected {exp}, got {val}");
    }
}

// ── Test 6: constant values survive downsampling ──────────────────────────────
#[test]
fn test_downsample_samples_constant() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        downsample_enabled: true,
        agg_mode: 1,
        agg_samples: 5,
        agg_stat: 0,
        std_filter_enabled: false,
        range_filter_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let n: i64 = conn
        .query_row("select count(*) from result", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, TEST2_DOWNSAMPLE_BUCKETS,
        "Expected {TEST2_DOWNSAMPLE_BUCKETS} downsampled rows, got {n}");

    let expected = [10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0];
    for (i, exp) in expected.iter().enumerate() {
        let ch = i + 1;
        let val: f64 = conn
            .query_row(
                &format!("select avg(eca{ch}) from result"),
                [], |r| r.get(0)
            ).unwrap();
        assert!((val - exp).abs() < 1e-6,
            "Downsample samples: eca{ch} expected {exp}, got {val}");
    }
}

#
[test]
fn test_downsample_time_constant() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        downsample_enabled: true,
        agg_mode: 0,
        agg_seconds: 1.0,
        agg_stat: 0,
        std_filter_enabled: false,
        range_filter_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let expected = [10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0];
    for (i, exp) in expected.iter().enumerate() {
        let ch = i + 1;
        let val: f64 = conn
            .query_row(
                &format!("select avg(eca{ch}) from result"),
                [], |r| r.get(0)
            ).unwrap();
        assert!((val - exp).abs() < 1e-6,
            "Downsample time: eca{ch} expected {exp}, got {val}");
    }
}

#[test]
fn test_downsample_median_constant() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        downsample_enabled: true,
        agg_mode: 1,
        agg_samples: 5,
        agg_stat: 1,
        std_filter_enabled: false,
        range_filter_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let expected = [10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0];
    for (i, exp) in expected.iter().enumerate() {
        let ch = i + 1;
        let val: f64 = conn
            .query_row(
                &format!("select avg(eca{ch}) from result"),
                [], |r| r.get(0)
            ).unwrap();
        assert!((val - exp).abs() < 1e-6,
            "Downsample median: eca{ch} expected {exp}, got {val}");
    }
}

// ── Test 7: std filter ────────────────────────────────────────────────────────
#[test]
fn test_std_filter_constant_no_removal() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        std_filter_enabled: true,
        std_threshold: 2.5,
        std_window_mode: 1,
        std_window_samples: 5,
        std_action: 1,
        range_filter_enabled: false,
        smooth_enabled: false,
        downsample_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let n: i64 = conn
        .query_row("select count(*) from result", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, TEST2_ROWS,
        "Constant data: std filter should not remove any rows, got {n}");
}

// ── Test 8: range filter ──────────────────────────────────────────────────────
#[test]
fn test_range_filter_flags_outside() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        range_filter_enabled: true,
        range_per_channel: true,
        range_min: [15.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        range_max: [100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
        range_action: 0,
        std_filter_enabled: false,
        smooth_enabled: false,
        downsample_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let null_eca1: i64 = conn
        .query_row(
            "select count(*) from result where eca1 is null",
            [], |r| r.get(0)
        ).unwrap();
    assert_eq!(null_eca1, TEST2_ROWS,
        "All eca1 should be null (out of range), got {null_eca1}");

    let null_eca2: i64 = conn
        .query_row(
            "select count(*) from result where eca2 is null",
            [], |r| r.get(0)
        ).unwrap();
    assert_eq!(null_eca2, 0,
        "eca2 should not be null (in range), got {null_eca2}");
}

#[test]
fn test_range_filter_removes_rows() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        range_filter_enabled: true,
        range_per_channel: false,
        range_global_min: 25.0,
        range_global_max: 100.0,
        range_action: 1,
        std_filter_enabled: false,
        smooth_enabled: false,
        downsample_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let n: i64 = conn
        .query_row("select count(*) from result", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0,
        "All rows removed when eca1 out of global range, got {n}");
}

// ── Test 9: GPS coordinates survive aggregation ───────────────────────────────
#[test]
fn test_coordinates_survive_smooth() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        smooth_enabled: true,
        agg_mode: 1,
        agg_samples: 5,
        agg_stat: 0,
        std_filter_enabled: false,
        range_filter_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let (lat, lon): (f64, f64) = conn
        .query_row(
            "select avg(latitude), avg(longitude) from result",
            [], |r| Ok((r.get(0)?, r.get(1)?))
        ).unwrap();
    assert!((lat - 45.0).abs() < 1e-6,
        "Latitude should survive smoothing as 45.0, got {lat}");
    assert!((lon - 12.0).abs() < 1e-6,
        "Longitude should survive smoothing as 12.0, got {lon}");
}

#[test]
fn test_coordinates_survive_downsample() {
    let conn = test_conn();
    let cfg = ProcessingConfig {
        downsample_enabled: true,
        agg_mode: 1,
        agg_samples: 5,
        agg_stat: 0,
        std_filter_enabled: false,
        range_filter_enabled: false,
        ..base_cfg()
    };
    run_pipeline(&conn, "tests/data/test2.txt", &cfg);

    let (lat, lon): (f64, f64) = conn
        .query_row(
            "select avg(latitude), avg(longitude) from result",
            [], |r| Ok((r.get(0)?, r.get(1)?))
        ).unwrap();
    assert!((lat - 45.0).abs() < 1e-6,
        "Latitude should survive downsampling as 45.0, got {lat}");
    assert!((lon - 12.0).abs() < 1e-6,
        "Longitude should survive downsampling as 12.0, got {lon}");
}
