//! Reproducible query/ranking evaluation using only isolated synthetic metadata.
//! No real media is generated or played and no user data directory is opened.
use std::{collections::HashSet, time::Instant};

use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sqlx::{QueryBuilder, Sqlite, sqlite::SqlitePoolOptions};
use vela_domain::{RecommendationFeedback, Recommendations};
use vela_media::MediaService;

#[tokio::main]
async fn main() -> Result<()> {
    let mut tracks = 10_000_u32;
    let mut iterations = 5_u32;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--tracks" => tracks = args.next().context("--tracks needs a count")?.parse()?,
            "--iterations" => {
                iterations = args.next().context("--iterations needs a count")?.parse()?
            }
            _ => anyhow::bail!(
                "usage: benchmark_recommendations [--tracks 1..1000000] [--iterations 1..30]"
            ),
        }
    }
    ensure!(
        (1..=1_000_000).contains(&tracks),
        "tracks must be 1..1000000"
    );
    ensure!((1..=30).contains(&iterations), "iterations must be 1..30");
    let directory = tempfile::tempdir()?;
    let db = directory.path().join("synthetic-recommendations.db");
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&db)
                .create_if_missing(true)
                .foreign_keys(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal),
        )
        .await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    sqlx::query("INSERT INTO users(id,username,password_hash) VALUES('benchmark-owner','benchmark','synthetic-no-auth')")
        .execute(&pool).await?;
    let seed_start = Instant::now();
    let mut tx = pool.begin().await?;
    for start in (0..tracks).step_by(500) {
        let end = (start + 500).min(tracks);
        let mut assets = QueryBuilder::<Sqlite>::new(
            "INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) ",
        );
        assets.push_values(start..end, |mut row, n| {
            row.push_bind(asset_id(n))
                .push_bind(format!("synthetic-hash-{n}"))
                .push_bind(format!("synthetic-no-audio/{n}"))
                .push_bind("audio/flac")
                .push_bind(1024_i64);
        });
        assets.build().execute(&mut *tx).await?;
        let mut catalog = QueryBuilder::<Sqlite>::new(
            "INSERT INTO tracks(id,owner_id,media_asset_id,title,artist,album,genre,duration_ms,codec,added_at) ",
        );
        catalog.push_values(start..end, |mut row, n| {
            row.push_bind(track_id(n))
                .push_bind("benchmark-owner")
                .push_bind(asset_id(n))
                .push_bind(format!("Synthetic track {n}"))
                .push_bind(format!("Synthetic artist {:03}", n % 64))
                .push_bind(format!("Synthetic album {:06}", n / 10))
                .push_bind(format!("Synthetic genre {}", n % 8))
                .push_bind(180_000_i64)
                .push_bind("FLAC")
                .push_bind(i64::from(n));
        });
        catalog.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let seeded_ms = seed_start.elapsed().as_secs_f64() * 1000.0;
    let media = MediaService::new(pool.clone(), directory.path().into(), 1024);
    let mut results = Vec::new();
    results.push(measure(&media, "cold-library-exploration", iterations).await?);
    for n in 0..tracks.min(12) {
        sqlx::query("INSERT INTO favorites(user_id,track_id) VALUES('benchmark-owner',?)")
            .bind(track_id(n))
            .execute(&pool)
            .await?;
    }
    results.push(measure(&media, "explicit-favorite-affinity", iterations).await?);
    sqlx::query("INSERT INTO listening_preferences(user_id,enabled,retention_days,revision,updated_at) VALUES('benchmark-owner',1,90,1,unixepoch())")
        .execute(&pool).await?;
    sqlx::query("INSERT INTO listening_devices(user_id,device_id,enabled) VALUES('benchmark-owner','00000000-0000-4000-8000-000000000001',1)")
        .execute(&pool).await?;
    for n in 0..tracks.min(20) {
        for occurrence in 0..=n % 4 {
            sqlx::query("INSERT INTO listening_sessions(user_id,session_id,device_id,track_id,sequence,listened_ms,position_ms,started_at,updated_at,qualified,ended) VALUES('benchmark-owner',?,'00000000-0000-4000-8000-000000000001',?,1,30000,30000,unixepoch()-60,unixepoch()-30,1,1)")
                .bind(uuid::Uuid::from_u128(1_000_000_000 + u128::from(n) * 4 + u128::from(occurrence)).to_string())
                .bind(track_id(n)).execute(&pool).await?;
        }
    }
    let mut hidden = HashSet::new();
    for n in tracks.saturating_sub(20)..tracks {
        media
            .set_recommendation_feedback(
                "benchmark-owner",
                RecommendationFeedback {
                    track_id: track_id(n),
                    hidden: true,
                },
            )
            .await?;
        hidden.insert(track_id(n));
    }
    let check = media.recommendations("benchmark-owner", 30).await?;
    ensure!(check.candidate_count <= 500 && check.anchor_count <= 12);
    ensure!(
        check
            .discovery
            .iter()
            .all(|item| !hidden.contains(&item.track.id))
    );
    ensure!(
        check
            .rotation
            .iter()
            .all(|item| !hidden.contains(&item.track.id))
    );
    let played: HashSet<_> = (0..tracks.min(20)).map(track_id).collect();
    ensure!(
        check
            .discovery
            .iter()
            .all(|item| !played.contains(&item.track.id))
    );
    ensure!(
        check
            .rotation
            .iter()
            .all(|item| played.contains(&item.track.id))
    );
    results.push(
        measure(
            &media,
            "qualified-private-history-and-exclusions",
            iterations,
        )
        .await?,
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "format_version":1,
            "scope":"isolated synthetic metadata; no source audio, physical playback, preference quality, public catalog or comparative superiority claim",
            "tracks":tracks,
            "iterations":iterations,
            "seed_ms":seeded_ms,
            "fixture":"64 synthetic artists, eight synthetic genre tags, ten tracks per album; current retained qualified sessions over up to20 tracks, twelve favorites, up to20 exclusions",
            "method":"one warmup then sequential on-demand reads; local SQLite WAL; latency excludes seeding; small-sample percentile is nearest rank, not tail-confidence evidence",
            "evaluation":results,
            "invariants":{"owner":"benchmark-owner only","candidate_bound":500,"anchor_bound":12,"history_exclusions":true,"hidden_exclusions":true,"rotation_uses_actual_fixture_sessions":true}
        }))?
    );
    pool.close().await;
    Ok(())
}

fn track_id(n: u32) -> String {
    uuid::Uuid::from_u128(1 + u128::from(n)).to_string()
}
fn asset_id(n: u32) -> String {
    uuid::Uuid::from_u128(10_000_001 + u128::from(n)).to_string()
}

async fn measure(media: &MediaService, name: &str, iterations: u32) -> Result<Value> {
    let warmup = media.recommendations("benchmark-owner", 30).await?;
    let expected = ids(&warmup);
    let mut ms = Vec::with_capacity(iterations as usize);
    let mut unique = HashSet::new();
    let mut final_result = warmup;
    for _ in 0..iterations {
        let start = Instant::now();
        final_result = media.recommendations("benchmark-owner", 30).await?;
        ms.push(start.elapsed().as_secs_f64() * 1000.0);
        ensure!(
            ids(&final_result) == expected,
            "non-deterministic IDs in fixed fixture"
        );
        unique.extend(
            final_result
                .discovery
                .iter()
                .map(|item| item.track.id.clone()),
        );
    }
    let mut sorted = ms.clone();
    sorted.sort_by(f64::total_cmp);
    let quantile = |p: f64| sorted[((p * sorted.len() as f64).ceil() as usize).saturating_sub(1)];
    let artists: HashSet<_> = final_result
        .discovery
        .iter()
        .map(|item| &item.track.artist)
        .collect();
    Ok(json!({
        "case":name,
        "latency_ms":{"samples":ms,"p50":quantile(0.5),"p95":quantile(0.95),"p99":quantile(0.99)},
        "examined_candidates":final_result.candidate_count,
        "anchors":final_result.anchor_count,
        "hidden":final_result.hidden_count,
        "discovery_tracks":final_result.discovery.len(),
        "rotation_tracks":final_result.rotation.len(),
        "unique_discovery_tracks_across_repeated_fixed_reads":unique.len(),
        "unique_discovery_artists":artists.len(),
        "examined_candidate_selection_fraction":if final_result.candidate_count==0 {0.0} else {unique.len() as f64/f64::from(final_result.candidate_count)},
        "unchanged_fixed_fixture_returns_stable_ids":true,
        "uses_audio_features":final_result.uses_audio_features
    }))
}

fn ids(result: &Recommendations) -> (Vec<String>, Vec<String>) {
    (
        result
            .discovery
            .iter()
            .map(|item| item.track.id.clone())
            .collect(),
        result
            .rotation
            .iter()
            .map(|item| item.track.id.clone())
            .collect(),
    )
}
