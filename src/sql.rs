use crate::processing::ProcessingConfig;

static REMOVE_FILTER: &str = "
    where eca1 is not null
        and eca2 is not null
        and eca3 is not null
        and eca4 is not null
        and eca5 is not null
        and eca6 is not null";

pub fn step_parse(datafile: &str) -> String {
    format!(r#"

create or replace table data as (
    select
        row_number() over () as rn,
        *
        from read_csv(
            '{datafile}',
            all_varchar=true,
            null_padding=true,
            sep=',',
            header=false
        )
        order by rn
);

create or replace table pgala as (
    select rn,
    column03 as emtime,
    column04 as bvolt,  column05 as bcur,  column06 as tempc,
    column07 as pitch,  column08 as roll,
    column09 as eca1,   column10 as msa1,
    column11 as eca2,   column12 as msa2,
    column13 as eca3,   column14 as msa3,
    column15 as eca4,   column16 as msa4,
    column17 as eca5,   column18 as msa5,
    column19 as eca6,   column20[0:-4] as msa6
    from data
    where column00 = '$PGALA'
    order by rn
);
create or replace table gngga as (
    select rn,
    column01 as gpstime,
    column02 as latitude,
    column04 as longitude,
    column09 as altitude,
    column06 as quality,
    column07 as numsv,
    column08 as hdop
    from data
    where column00 = '$GNGGA'
    order by rn
);
create or replace table gnzda as (
    select rn,
    strptime(
        column04 || column03 || column02 || column01,
        '%Y%m%d%H%M%S.%f'
    ) as date
    from data
    where column00 = '$GNZDA'
    order by rn
);
    "#, datafile = datafile)
}

pub fn step_join(use_gps: bool) -> String {
    if use_gps {r#"
create or replace table data as (
    select
        pgala.rn as emrn,
        gngga.rn as ggarn,
        case when (gnzdafwd.rn - pgala.rn) > (pgala.rn - gnzdabwd.rn)
            then gnzdabwd.rn
            else ifnull(gnzdafwd.rn, gnzdabwd.rn)
            end as zdarn,
        cast(strptime(pgala.emtime, '%H%M%S.%f') as time) as emtime,
        floor(extract(epoch from cast(strptime(pgala.emtime, '%H%M%S.%f') as time)) * 1000)::bigint as emtime_ms,
        cast(strptime(gngga.gpstime, '%H%M%S.%f') as time) as gpstime,
        case when (gnzdafwd.rn - pgala.rn) > (pgala.rn - gnzdabwd.rn)
            then gnzdabwd.date
            else ifnull(gnzdafwd.date, gnzdabwd.date)
            end as zdadate,
        (case when (gnzdafwd.rn - pgala.rn) > (pgala.rn - gnzdabwd.rn)
            then gnzdabwd.date
            else ifnull(gnzdafwd.date, gnzdabwd.date)
        end)::date +
            cast(strptime(gngga.gpstime, '%H%M%S.%f') as time) as gpsts,
        cast(gngga.latitude[:2]  as double) +
            (cast(gngga.latitude[3:]  as double) / 60) as latitude,
        cast(gngga.longitude[:3] as double) +
            (cast(gngga.longitude[4:] as double) / 60) as longitude,
        cast(gngga.quality as integer) as quality,
        cast(gngga.numsv  as integer) as numsv,
        cast(gngga.hdop   as double)  as hdop,
        cast(pgala.bvolt  as double)  as bvolt,
        cast(pgala.bcur   as double)  as bcur,
        cast(pgala.tempc  as double)  as tempc,
        cast(pgala.pitch  as double)  as pitch,
        cast(pgala.roll   as double)  as roll,
        cast(pgala.eca1   as double)  as eca1,
        cast(pgala.msa1   as double)  as msa1,
        cast(pgala.eca2   as double)  as eca2,
        cast(pgala.msa2   as double)  as msa2,
        cast(pgala.eca3   as double)  as eca3,
        cast(pgala.msa3   as double)  as msa3,
        cast(pgala.eca4   as double)  as eca4,
        cast(pgala.msa4   as double)  as msa4,
        cast(pgala.eca5   as double)  as eca5,
        cast(pgala.msa5   as double)  as msa5,
        cast(pgala.eca6   as double)  as eca6,
        cast(pgala.msa6   as double)  as msa6
    from pgala
    asof left join gngga on pgala.rn <= gngga.rn
    asof left join gnzda as gnzdafwd on pgala.rn <= gnzdafwd.rn
    asof left join gnzda as gnzdabwd on pgala.rn >= gnzdabwd.rn
    order by pgala.rn
);
        "#.to_string()
    } else {r#"
create or replace table data as (
    select
        pgala.rn as emrn,
        cast(strptime(pgala.emtime, '%H%M%S.%f') as time) as emtime,
        floor(extract(epoch from
            cast(strptime(pgala.emtime, '%H%M%S.%f') as time)
        ) * 1000)::bigint as emtime_ms,
        null::time      as gpstime,
        null::timestamp as gpsts,
        null::timestamp as zdadate,
        null::double    as latitude,
        null::double    as longitude,
        null::integer   as quality,
        null::integer   as numsv,
        null::double    as hdop,
        cast(pgala.bvolt as double) as bvolt,
        cast(pgala.bcur  as double) as bcur,
        cast(pgala.tempc as double) as tempc,
        cast(pgala.pitch as double) as pitch,
        cast(pgala.roll  as double) as roll,
        cast(pgala.eca1  as double) as eca1,
        cast(pgala.msa1  as double) as msa1,
        cast(pgala.eca2  as double) as eca2,
        cast(pgala.msa2  as double) as msa2,
        cast(pgala.eca3  as double) as eca3,
        cast(pgala.msa3  as double) as msa3,
        cast(pgala.eca4  as double) as eca4,
        cast(pgala.msa4  as double) as msa4,
        cast(pgala.eca5  as double) as eca5,
        cast(pgala.msa5  as double) as msa5,
        cast(pgala.eca6  as double) as eca6,
        cast(pgala.msa6  as double) as msa6
    from pgala
    order by pgala.rn
);
        "#.to_string()
    }
}

pub fn step_set_nan_to_null() -> &'static str {"
create or replace table data as (
    select
        * 
        replace(
            case when isnan(eca1) then null else eca1 end as eca1,
            case when isnan(eca2) then null else eca2 end as eca2,
            case when isnan(eca3) then null else eca3 end as eca3,
            case when isnan(eca4) then null else eca4 end as eca4,
            case when isnan(eca5) then null else eca5 end as eca5,
            case when isnan(eca6) then null else eca6 end as eca6
        )
    from data
    order by emrn
);
"
}

pub fn step_check_nulls() -> &'static str {
    "select column_name, null_percentage from (summarize select eca1, eca2, eca3, eca4, eca5, eca6 from data);"
}

pub fn step_validate_row_count() -> &'static str {
    "select count(*) as n from data;"
}

pub fn step_validate_gps() -> &'static str {r#"
select
    count(*) as total,
    count(*) filter (where latitude  is null
                        or longitude is null
                        or quality = 0)         as null_gps,
    count(*) filter (where numsv < 8)           as low_numsv,
    count(*) filter (where hdop  > 2.0)         as high_hdop,
    count(*) filter (where abs(pitch) > 5.0)    as high_pitch,
    count(*) filter (where abs(roll)  > 5.0)    as high_roll
from data;
    "#
}

pub fn step_validate_gnzda() -> &'static str {
    "select count(*) as n from gnzda;"
}

pub fn step_validate_gngga() -> &'static str {
    "select count(*) as n from gngga;"
}

pub fn step_range_filter(cfg: &ProcessingConfig) -> String {

    let eca_range_filter = |ch: usize| -> String {
        let (mn, mx) = if cfg.range_per_channel {
            (cfg.range_min[ch - 1], cfg.range_max[ch - 1])
        } else {
            (cfg.range_global_min, cfg.range_global_max)
        };
        format!("case when eca{ch} between {mn} and {mx} then j.eca{ch} else null end as eca{ch}")
    };

    let eca_with_range_conds = (1..=6)
        .map(|i| eca_range_filter(i))
        .collect::<Vec<_>>()
        .join(",\n        ");

    let remove_filter = if cfg.range_action == 1 {
        REMOVE_FILTER
    } else {
        ""
    };

    format!(r#"
create or replace table data as (
    with data_null as (
        select
            j.emrn, j.emtime, j.emtime_ms,
            j.gpstime, j.gpsts, j.zdadate,
            j.latitude, j.longitude,
            j.quality, j.numsv, j.hdop,
            j.bvolt, j.bcur, j.tempc,
            j.pitch, j.roll,
            j.msa1, j.msa2, j.msa3,
            j.msa4, j.msa5, j.msa6,
            {eca_with_range_conds}
        from data as j
    )
    select * from data_null {remove_filter} order by emrn
    );"#)
}

pub fn step_std_stats(cfg: &ProcessingConfig) -> String {
    let time_col = if cfg.use_gps { "gpsts" } else { "emtime_ms" };

    let window_def = match cfg.std_window_mode {
        0 => if cfg.use_gps {
            format!(
                "window w as (order by {time_col} range between \
                 interval {} seconds preceding and \
                 interval {} seconds following)",
                cfg.std_window_seconds, cfg.std_window_seconds
            )
        } else {
            let ms = (cfg.std_window_seconds * 1000.0) as i64;
            format!(
                "window w as (order by {time_col} range between \
                 {ms} preceding and {ms} following)"
            )
        },
        _ => format!(
            "window w as (order by emrn rows between {} preceding and {} following)",
            cfg.std_window_samples / 2,
            cfg.std_window_samples / 2
        ),
    };

    format!(r#"
        create or replace table std_stats as (
            select
                emrn,
                stddev(eca1) over w as stddev1, mean(eca1) over w as mean1,
                stddev(eca2) over w as stddev2, mean(eca2) over w as mean2,
                stddev(eca3) over w as stddev3, mean(eca3) over w as mean3,
                stddev(eca4) over w as stddev4, mean(eca4) over w as mean4,
                stddev(eca5) over w as stddev5, mean(eca5) over w as mean5,
                stddev(eca6) over w as stddev6, mean(eca6) over w as mean6
            from data
            {window_def}
        );
            "#)
}


pub fn step_std_filter(cfg: &ProcessingConfig) -> String {
    let closure_std_filter = |ch: usize| -> String {
        format!("case when abs(eca{ch} - mean{ch}) <= ((stddev{ch} * {}) + 0.001) then j.eca{ch} else null end as eca{ch}",
            cfg.std_threshold)
    };
    let eca_with_std_conds = (1..=6)
        .map(|i| closure_std_filter(i))
        .collect::<Vec<_>>()
        .join(",\n                ");
        
    let remove_filter = if cfg.std_action == 1 {
        REMOVE_FILTER
    } else {
        ""
    };

    format!(r#"
        create or replace table data as (
            with data_null as (
                select
                    j.emrn, j.emtime, j.emtime_ms,
                    j.gpstime, j.gpsts, j.zdadate,
                    j.latitude, j.longitude,
                    j.quality, j.numsv, j.hdop,
                    j.bvolt, j.bcur, j.tempc,
                    j.pitch, j.roll,
                    j.msa1, j.msa2, j.msa3,
                    j.msa4, j.msa5, j.msa6,
                    s.stddev1, s.mean1, s.stddev2, s.mean2,
                    s.stddev3, s.mean3, s.stddev4, s.mean4,
                    s.stddev5, s.mean5, s.stddev6, s.mean6,
                    {eca_with_std_conds}
                from data j left join std_stats s on j.emrn = s.emrn
            )
        select * from data_null {remove_filter} order by emrn
        );
    "#)
}

pub fn step_validate_filter() -> &'static str {
    r#"
select
    count(*) as total,
    count(*) filter (where eca1 is null
                        or eca2 is null
                        or eca3 is null
                        or eca4 is null
                        or eca5 is null
                        or eca6 is null) as flagged
from data;
    "#
}

pub fn step_smooth(cfg: &ProcessingConfig) -> String {
    let stat = if cfg.agg_stat == 0 { "mean" } else { "median" };
    let time_col = if cfg.use_gps { "gpsts" } else { "emtime_ms" };

    let window_def = match cfg.agg_mode {
        0 => if cfg.use_gps {
            format!(
                "window sw as (order by {time_col} range between \
                 interval {ms} milliseconds preceding and \
                 interval {ms} milliseconds following)",
                ms = (cfg.agg_seconds * 1000.0) as i64,
            )
        } else {
            let ms = (cfg.agg_seconds * 1000.0) as i64;
            format!(
                "window sw as (order by {time_col} range between \
                 {ms} preceding and {ms} following)"
            )
        },
        _ => format!(
            "window sw as (order by emrn rows between {} preceding and {} following)",
            cfg.agg_samples / 2,
            cfg.agg_samples / 2
        ),
    };

    format!(r#"
create or replace table data as (
    select
        emrn, emtime, emtime_ms,
        gpstime, gpsts, zdadate,
        latitude, longitude,
        quality, numsv, hdop,
        {stat}(bvolt) over sw as bvolt,
        {stat}(bcur)  over sw as bcur,
        {stat}(tempc) over sw as tempc,
        degrees(atan2(
            mean(sin(radians(pitch))) over sw,
            mean(cos(radians(pitch))) over sw
        )) as pitch,
        degrees(atan2(
            mean(sin(radians(roll))) over sw,
            mean(cos(radians(roll))) over sw
        )) as roll,
        {stat}(eca1) over sw as eca1, {stat}(msa1) over sw as msa1,
        {stat}(eca2) over sw as eca2, {stat}(msa2) over sw as msa2,
        {stat}(eca3) over sw as eca3, {stat}(msa3) over sw as msa3,
        {stat}(eca4) over sw as eca4, {stat}(msa4) over sw as msa4,
        {stat}(eca5) over sw as eca5, {stat}(msa5) over sw as msa5,
        {stat}(eca6) over sw as eca6, {stat}(msa6) over sw as msa6
    from data
    {window_def}
    order by emrn
);
    "#)
}

pub fn step_downsample(cfg: &ProcessingConfig) -> String {
    let stat = if cfg.agg_stat == 0 { "mean" } else { "median" };
    let time_col = if cfg.use_gps { "gpsts" } else { "emtime_ms" };

    let gps_cols = if cfg.use_gps {
        format!("
            {stat}(gpstime) as gpstime,
            {stat}(gpsts) as gpsts,
            {stat}(latitude) as latitude,
            {stat}(longitude) as longitude,
            {stat}(quality) as quality,
            {stat}(numsv) as numsv,
            {stat}(hdop) as hdop,"
        )
    } else {"
        null::time as gpstime,
        null::timestamp as gpsts,
        null::timestamp as zdadate,
        null::double as latitude,
        null::double as longitude,
        null::integer as quality,
        null::integer as numsv,
        null::double as hdop,".to_string()
    };

    // 0 => time_bucket, else row number
    match cfg.agg_mode {
        0 => {
            let ms = (cfg.agg_seconds * 1000.0) as i64;

            let bucket = if cfg.use_gps {
                format!("time_bucket(interval '{ms} milliseconds', gpsts)")
            } else {
                format!("floor({time_col} / {ms})")
            };

            format!(r#"
                create or replace table data as (
                    select
                        {bucket} as bucket,
                        {stat}(emtime) as emtime,
                        {gps_cols}
                        {stat}(bvolt) as bvolt,
                        {stat}(bcur)  as bcur,
                        {stat}(tempc) as tempc,
                        degrees(atan2(mean(sin(radians(pitch))), mean(cos(radians(pitch))))) as pitch,
                        degrees(atan2(mean(sin(radians(roll))), mean(cos(radians(roll))))) as roll,
                        {stat}(eca1) as eca1, {stat}(msa1) as msa1,
                        {stat}(eca2) as eca2, {stat}(msa2) as msa2,
                        {stat}(eca3) as eca3, {stat}(msa3) as msa3,
                        {stat}(eca4) as eca4, {stat}(msa4) as msa4,
                        {stat}(eca5) as eca5, {stat}(msa5) as msa5,
                        {stat}(eca6) as eca6, {stat}(msa6) as msa6
                    from data
                    group by bucket
                    order by bucket
                );
            "#)
        },
        _ => format!(r#"
            create or replace table data as (
                with data_bucketed as (
                    select *,
                    floor((row_number() over (order by emrn) - 1) / {n}) as bucket
                    from data
                )
                select
                    bucket,
                    {stat}(emtime) as emtime,
                    {gps_cols}
                    {stat}(bvolt) as bvolt,
                    {stat}(bcur)  as bcur,
                    {stat}(tempc) as tempc,
                    degrees(atan2(
                        mean(sin(radians(pitch))),
                        mean(cos(radians(pitch)))
                    )) as pitch,
                    degrees(atan2(
                        mean(sin(radians(roll))),
                        mean(cos(radians(roll)))
                    )) as roll,
                    {stat}(eca1) as eca1, {stat}(msa1) as msa1,
                    {stat}(eca2) as eca2, {stat}(msa2) as msa2,
                    {stat}(eca3) as eca3, {stat}(msa3) as msa3,
                    {stat}(eca4) as eca4, {stat}(msa4) as msa4,
                    {stat}(eca5) as eca5, {stat}(msa5) as msa5,
                    {stat}(eca6) as eca6, {stat}(msa6) as msa6
                from data_bucketed
                group by bucket
                order by bucket
            );
        "#, n = cfg.agg_samples),
    }
}

pub fn step_output(table: &str, cfg: &ProcessingConfig) -> String {
    let freq   = &cfg.freq;
    let height = &cfg.height;
    let cf = [225.1582_f64, 114.8766, 69.4933, 46.5203, 25.0176, 14.8033];
    let seps = [0.5_f64, 0.7, 0.9, 1.1, 1.5, 1.95];

    let emagpy_cols = if cfg.out_emagpy {
        let cols = (0..6)
            .map(|i| format!(
                r#"round(eca{ch} / {cf}, 6) as "HCP{sep}f{freq}h{height}_quad""#,
                ch = i + 1, cf = cf[i], sep = seps[i],
                freq = freq, height = height,
            ))
            .collect::<Vec<_>>()
            .join(",\n        ");
        format!(",\n        {cols}")
    } else {
        String::new()
    };

    let order_col = if cfg.downsample_enabled { "bucket" } else { "emrn" };

    format!(r#"
create or replace table {table} as (
    select *{emagpy_cols}
    from data
    order by {order_col}
);
    "#)
}

// ── CSV export ────────────────────────────────────────────────────────────────
pub fn copy_to_csv(table: &str, outfile: &str) -> String {
    format!("copy {table} to '{outfile}' (header, delimiter ',');")
}
