//! Real SQLite query fixtures plus actual encoded-audio ingestion for filters.
use std::{collections::HashSet, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tempfile::TempDir;
use uuid::Uuid;
use vela_domain::{TrackFilters, TrackSort};
use vela_media::{InvalidTrackFilter, MediaService};

struct Fixture {
    directory: TempDir,
    pool: SqlitePool,
    media: MediaService,
}

struct Input<'a> {
    owner: &'a str,
    title: &'a str,
    genre: &'a str,
    codec: &'a str,
    year: Option<i64>,
    bitrate: Option<i64>,
    sample_rate: Option<i64>,
    bit_depth: Option<i64>,
    added_at: i64,
}

impl Default for Input<'_> {
    fn default() -> Self {
        Self {
            owner: "alice",
            title: "Track",
            genre: "Jazz",
            codec: "FLAC",
            year: Some(2026),
            bitrate: Some(1000),
            sample_rate: Some(48000),
            bit_depth: Some(24),
            added_at: 100,
        }
    }
}

impl Fixture {
    async fn new(filter_indexes: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let options = SqliteConnectOptions::new()
            .filename(directory.path().join("vela.db"))
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();
        for migration in [
            include_str!("../../../migrations/0001_initial.sql"),
            include_str!("../../../migrations/0002_catalog_groups.sql"),
            include_str!("../../../migrations/0003_catalog_metadata_revision.sql"),
            include_str!("../../../migrations/0013_stats_covering_indexes.sql"),
            include_str!("../../../migrations/0004_readonly_scan_sources.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        if filter_indexes {
            sqlx::raw_sql(include_str!("../../../migrations/0005_catalog_filters.sql"))
                .execute(&pool)
                .await
                .unwrap();
            sqlx::raw_sql(include_str!(
                "../../../migrations/0006_cover_genre_facets.sql"
            ))
            .execute(&pool)
            .await
            .unwrap();
        }
        for owner in ["alice", "bob"] {
            sqlx::query("INSERT INTO users(id,username,password_hash) VALUES(?,?,?)")
                .bind(owner)
                .bind(owner)
                .bind("fixture hash")
                .execute(&pool)
                .await
                .unwrap();
        }
        let media = MediaService::new(pool.clone(), directory.path().to_owned(), 1024 * 1024);
        media.initialize().await.unwrap();
        Self {
            directory,
            pool,
            media,
        }
    }

    async fn add(&self, number: u128, input: Input<'_>) -> String {
        let id = Uuid::from_u128(number).to_string();
        let hash = blake3::hash(id.as_bytes()).to_hex().to_string();
        // Metadata-only fixtures model existing catalog rows, never playable audio.
        sqlx::query("INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) VALUES(?,?,?,'audio/flac',8)")
            .bind(&id).bind(&hash).bind(format!("media/{hash}.audio"))
            .execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,genre,codec,year,bitrate,sample_rate,bit_depth,duration_ms,added_at) VALUES(?,?,?,?,?,?,?,?,?,?,1000,?)")
            .bind(&id).bind(input.owner).bind(&id).bind(input.title).bind(input.genre)
            .bind(input.codec).bind(input.year).bind(input.bitrate).bind(input.sample_rate)
            .bind(input.bit_depth).bind(input.added_at).execute(&self.pool).await.unwrap();
        id
    }

    async fn ids(&self, filters: &TrackFilters) -> Vec<String> {
        self.media
            .list_tracks_filtered("alice", None, None, 100, false, filters)
            .await
            .unwrap()
            .items
            .into_iter()
            .map(|track| track.id)
            .collect()
    }
}

#[tokio::test]
async fn actual_flac_properties_combine_with_search_favorites_and_owner() {
    let fixture = Fixture::new(true).await;
    let path = fixture.directory.path().join("staging/real.flac");
    tokio::fs::write(&path, include_bytes!("fixtures/signal.flac"))
        .await
        .unwrap();
    let track = fixture
        .media
        .ingest("alice", &path, "signal.flac")
        .await
        .unwrap()
        .track;
    let filters = TrackFilters {
        genre: Some(track.genre.to_ascii_lowercase()),
        codec: Some(" flac ".to_owned()),
        year_from: track.year,
        year_to: track.year,
        min_bitrate: track.bitrate,
        min_sample_rate: track.sample_rate,
        min_bit_depth: track.bit_depth,
        sort: TrackSort::Title,
    };
    assert!(
        fixture
            .media
            .list_tracks_filtered("alice", Some("sig"), None, 100, true, &filters)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    fixture
        .media
        .set_favorite("alice", &track.id, true)
        .await
        .unwrap();
    let page = fixture
        .media
        .list_tracks_filtered("alice", Some("sig"), None, 100, true, &filters)
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, track.id);
    assert!(page.items[0].favorite);
    assert!(
        fixture
            .media
            .list_tracks_filtered("bob", Some("sig"), None, 100, false, &filters)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let facets = fixture.media.catalog_facets("alice").await.unwrap();
    assert_eq!(facets.genres, vec![track.genre.trim()]);
    assert_eq!(facets.codecs, vec!["FLAC"]);
    assert!(!facets.genres_truncated && !facets.codecs_truncated);
    assert_eq!(
        tokio::fs::read(path).await.unwrap(),
        include_bytes!("fixtures/signal.flac")
    );
}

#[tokio::test]
async fn literal_filters_match_complete_tags_and_exclude_unknown_properties() {
    let fixture = Fixture::new(true).await;
    let known = fixture.add(1, Input::default()).await;
    fixture
        .add(
            2,
            Input {
                genre: "Jazz; Blues",
                ..Input::default()
            },
        )
        .await;
    let unknown = fixture
        .add(
            3,
            Input {
                year: None,
                bitrate: None,
                sample_rate: None,
                bit_depth: None,
                ..Input::default()
            },
        )
        .await;
    let untrusted = "Jazz' OR 1=1 -- %_";
    let literal = fixture
        .add(
            4,
            Input {
                genre: untrusted,
                codec: "FLAC' OR 1=1 --",
                ..Input::default()
            },
        )
        .await;
    for filters in [
        TrackFilters {
            genre: Some(format!(" {untrusted} ")),
            ..TrackFilters::default()
        },
        TrackFilters {
            codec: Some("FLAC' OR 1=1 --".to_owned()),
            ..TrackFilters::default()
        },
    ] {
        assert_eq!(fixture.ids(&filters).await, vec![literal.clone()]);
    }
    let jazz = fixture
        .ids(&TrackFilters {
            genre: Some(" jazz ".to_owned()),
            ..TrackFilters::default()
        })
        .await;
    assert_eq!(jazz, vec![known.clone(), unknown.clone()]);
    for criterion in [
        TrackFilters {
            year_from: Some(2026),
            ..TrackFilters::default()
        },
        TrackFilters {
            year_to: Some(2026),
            ..TrackFilters::default()
        },
        TrackFilters {
            min_bitrate: Some(1000),
            ..TrackFilters::default()
        },
        TrackFilters {
            min_sample_rate: Some(48000),
            ..TrackFilters::default()
        },
        TrackFilters {
            min_bit_depth: Some(24),
            ..TrackFilters::default()
        },
    ] {
        let ids = fixture.ids(&criterion).await;
        assert!(ids.contains(&known));
        assert!(!ids.contains(&unknown));
    }
    assert!(
        fixture
            .ids(&TrackFilters {
                min_bitrate: Some(1001),
                ..TrackFilters::default()
            })
            .await
            .is_empty()
    );
    assert_eq!(fixture.media.stats("alice").await.unwrap().tracks, 4);
}

#[tokio::test]
async fn all_sort_orders_page_case_and_time_ties_without_duplicates() {
    let fixture = Fixture::new(true).await;
    let one = fixture
        .add(
            1,
            Input {
                title: "beta",
                added_at: 101,
                ..Input::default()
            },
        )
        .await;
    let two = fixture
        .add(
            2,
            Input {
                title: "Alpha",
                ..Input::default()
            },
        )
        .await;
    let three = fixture
        .add(
            3,
            Input {
                title: "alpha",
                ..Input::default()
            },
        )
        .await;
    let four = fixture
        .add(
            4,
            Input {
                title: "BETA",
                added_at: 101,
                ..Input::default()
            },
        )
        .await;
    let five = fixture
        .add(
            5,
            Input {
                title: "zulu",
                added_at: 99,
                ..Input::default()
            },
        )
        .await;
    fixture
        .add(
            6,
            Input {
                owner: "bob",
                title: "Alpha",
                added_at: 102,
                ..Input::default()
            },
        )
        .await;
    for (sort, expected) in [
        (TrackSort::Library, vec![&one, &two, &three, &four, &five]),
        (TrackSort::Title, vec![&two, &three, &one, &four, &five]),
        (TrackSort::Newest, vec![&four, &one, &three, &two, &five]),
    ] {
        let filters = TrackFilters {
            sort,
            ..TrackFilters::default()
        };
        for page_size in [1, 2, 3] {
            let mut cursor = None;
            let mut ids = Vec::new();
            loop {
                let page = fixture
                    .media
                    .list_tracks_filtered(
                        "alice",
                        None,
                        cursor.as_deref(),
                        page_size,
                        false,
                        &filters,
                    )
                    .await
                    .unwrap();
                ids.extend(page.items.into_iter().map(|track| track.id));
                cursor = page.next_cursor;
                if cursor.is_none() {
                    break;
                }
                assert!(ids.len() <= expected.len(), "cursor must advance");
            }
            assert_eq!(ids.iter().collect::<Vec<_>>(), expected);
            assert_eq!(ids.iter().collect::<HashSet<_>>().len(), expected.len());
        }
    }
}

#[tokio::test]
async fn sorted_cursors_authorize_before_empty_search_and_accept_owned_sort_keys() {
    let fixture = Fixture::new(true).await;
    let owned = fixture
        .add(
            1,
            Input {
                genre: "Rock",
                ..Input::default()
            },
        )
        .await;
    let private = fixture
        .add(
            2,
            Input {
                owner: "bob",
                ..Input::default()
            },
        )
        .await;
    let absent = Uuid::from_u128(3).to_string();
    for sort in [TrackSort::Title, TrackSort::Newest] {
        let filters = TrackFilters {
            genre: Some("Jazz".to_owned()),
            sort,
            ..TrackFilters::default()
        };
        for query in [None, Some("***"), Some("definitelyabsenttoken")] {
            for cursor in [private.as_str(), absent.as_str(), "bad UUID"] {
                let error = fixture
                    .media
                    .list_tracks_filtered("alice", query, Some(cursor), 100, false, &filters)
                    .await
                    .unwrap_err();
                assert!(error.is::<InvalidTrackFilter>());
                assert_eq!(error.to_string(), "Invalid track cursor");
            }
            // Membership in the owner library is sufficient: a changed draft
            // can legitimately exclude the anchor from the filtered result.
            assert!(
                fixture
                    .media
                    .list_tracks_filtered("alice", query, Some(&owned), 100, false, &filters)
                    .await
                    .unwrap()
                    .items
                    .is_empty()
            );
        }
    }
    // Library order remains UUID-index seek without requiring cursor existence.
    assert!(
        fixture
            .media
            .list_tracks("alice", None, Some(&private), 100, false)
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn facets_are_trimmed_case_collapsed_sorted_owner_private_and_bounded() {
    let fixture = Fixture::new(true).await;
    for (number, genre, codec) in [
        (1, " jazz ", "flac"),
        (2, "JAZZ", "FLAC"),
        (3, "Ambient; Jazz", "MPEG audio"),
        (4, " ", ""),
    ] {
        fixture
            .add(
                number,
                Input {
                    genre,
                    codec,
                    ..Input::default()
                },
            )
            .await;
    }
    fixture
        .add(
            5,
            Input {
                owner: "bob",
                genre: "PRIVATE GENRE",
                codec: "PRIVATE FORMAT",
                ..Input::default()
            },
        )
        .await;
    let facets = fixture.media.catalog_facets("alice").await.unwrap();
    assert_eq!(facets.genres, vec!["Ambient; Jazz", "JAZZ"]);
    assert_eq!(facets.codecs, vec!["FLAC", "MPEG audio"]);
    assert!(!facets.genres_truncated && !facets.codecs_truncated);
    let private = fixture.media.catalog_facets("bob").await.unwrap();
    assert_eq!(private.genres, vec!["PRIVATE GENRE"]);
    assert_eq!(private.codecs, vec!["PRIVATE FORMAT"]);
    let empty = fixture.media.catalog_facets("unknown user").await.unwrap();
    assert!(empty.genres.is_empty() && empty.codecs.is_empty());

    for number in 0..102 {
        fixture
            .add(
                100 + number,
                Input {
                    genre: &format!("Genre {number:03}"),
                    codec: &format!("Codec {number:03}"),
                    ..Input::default()
                },
            )
            .await;
    }
    let bounded = fixture.media.catalog_facets("alice").await.unwrap();
    assert_eq!(bounded.genres.len(), 100);
    assert_eq!(bounded.codecs.len(), 32);
    assert!(bounded.genres_truncated && bounded.codecs_truncated);
    assert_eq!(bounded.genres.first().unwrap(), "Ambient; Jazz");
    assert_eq!(bounded.genres.last().unwrap(), "Genre 098");
    assert_eq!(bounded.codecs.first().unwrap(), "Codec 000");
    assert_eq!(bounded.codecs.last().unwrap(), "Codec 031");
}

#[tokio::test]
async fn invalid_filter_boundaries_are_typed_and_safe_even_with_no_search_matches() {
    let fixture = Fixture::new(true).await;
    let invalid_filters = [
        TrackFilters {
            genre: Some(" ".to_owned()),
            ..TrackFilters::default()
        },
        TrackFilters {
            genre: Some("a".repeat(1025)),
            ..TrackFilters::default()
        },
        TrackFilters {
            genre: Some("Jazz\n".to_owned()),
            ..TrackFilters::default()
        },
        TrackFilters {
            codec: Some("".to_owned()),
            ..TrackFilters::default()
        },
        TrackFilters {
            codec: Some("a".repeat(65)),
            ..TrackFilters::default()
        },
        TrackFilters {
            codec: Some("FLAC\0".to_owned()),
            ..TrackFilters::default()
        },
        TrackFilters {
            year_from: Some(0),
            ..TrackFilters::default()
        },
        TrackFilters {
            year_to: Some(10000),
            ..TrackFilters::default()
        },
        TrackFilters {
            year_from: Some(2026),
            year_to: Some(2025),
            ..TrackFilters::default()
        },
        TrackFilters {
            min_bitrate: Some(0),
            ..TrackFilters::default()
        },
        TrackFilters {
            min_bitrate: Some(1_000_001),
            ..TrackFilters::default()
        },
        TrackFilters {
            min_sample_rate: Some(0),
            ..TrackFilters::default()
        },
        TrackFilters {
            min_sample_rate: Some(1_536_001),
            ..TrackFilters::default()
        },
        TrackFilters {
            min_bit_depth: Some(0),
            ..TrackFilters::default()
        },
        TrackFilters {
            min_bit_depth: Some(65),
            ..TrackFilters::default()
        },
    ];
    for filters in invalid_filters {
        let error = fixture
            .media
            .list_tracks_filtered(
                "alice",
                Some("definitelyabsenttoken"),
                None,
                100,
                false,
                &filters,
            )
            .await
            .unwrap_err();
        assert!(error.is::<InvalidTrackFilter>(), "{error:?}");
        assert!(error.to_string().len() < 80);
        assert!(!error.to_string().contains('\n'));
    }
    for (query, cursor) in [(Some("a".repeat(257)), None), (None, Some("a".repeat(129)))] {
        assert!(
            fixture
                .media
                .list_tracks_filtered(
                    "alice",
                    query.as_deref(),
                    cursor.as_deref(),
                    100,
                    false,
                    &TrackFilters::default()
                )
                .await
                .unwrap_err()
                .is::<InvalidTrackFilter>()
        );
    }
    // Inclusive extremes are accepted even though this catalog has no matches.
    for filters in [
        TrackFilters {
            genre: Some("a".repeat(1024)),
            codec: Some("a".repeat(64)),
            year_from: Some(1),
            year_to: Some(9999),
            min_bitrate: Some(1_000_000),
            min_sample_rate: Some(1_536_000),
            min_bit_depth: Some(64),
            ..TrackFilters::default()
        },
        TrackFilters {
            year_from: Some(1),
            year_to: Some(1),
            min_bitrate: Some(1),
            min_sample_rate: Some(1),
            min_bit_depth: Some(1),
            ..TrackFilters::default()
        },
    ] {
        assert!(fixture.ids(&filters).await.is_empty());
    }
    fixture
        .media
        .list_tracks("alice", Some(&"a".repeat(256)), None, 100, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn default_wrapper_and_bounds_work_without_the_new_filter_indexes() {
    let fixture = Fixture::new(false).await;
    for number in 1..=102 {
        fixture.add(number, Input::default()).await;
    }
    let default = fixture
        .media
        .list_tracks("alice", None, None, u32::MAX, false)
        .await
        .unwrap();
    let filtered = fixture
        .media
        .list_tracks_filtered(
            "alice",
            None,
            None,
            u32::MAX,
            false,
            &TrackFilters::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        default
            .items
            .iter()
            .map(|track| &track.id)
            .collect::<Vec<_>>(),
        filtered
            .items
            .iter()
            .map(|track| &track.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(default.next_cursor, filtered.next_cursor);
    assert_eq!(filtered.items.len(), 100);
    let next = fixture
        .media
        .list_tracks("alice", None, filtered.next_cursor.as_deref(), 100, false)
        .await
        .unwrap();
    assert_eq!(next.items.len(), 2);
    assert!(next.next_cursor.is_none());
    assert_eq!(
        fixture
            .media
            .list_tracks("alice", None, None, 0, false)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    for sort in [TrackSort::Title, TrackSort::Newest] {
        assert_eq!(
            fixture
                .media
                .list_tracks_filtered(
                    "alice",
                    None,
                    None,
                    0,
                    false,
                    &TrackFilters {
                        sort,
                        ..TrackFilters::default()
                    }
                )
                .await
                .unwrap()
                .items
                .len(),
            1
        );
    }
    assert_eq!(
        fixture.media.catalog_facets("alice").await.unwrap().genres,
        vec!["Jazz"]
    );
}

#[tokio::test]
async fn broad_fts_branch_combines_typed_clauses_and_sorted_page_cursors() {
    let fixture = Fixture::new(true).await;
    // One transaction seeds >4096 genuine FTS entries, selecting the adaptive
    // broad-search branch rather than replacing the search engine with a mock.
    let mut transaction = fixture.pool.begin().await.unwrap();
    sqlx::query("WITH RECURSIVE entries(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM entries WHERE n<4100) INSERT INTO media_assets(id,hash,storage_path,content_type,size_bytes) SELECT printf('%08x-0000-4000-8000-%012x',n,n),printf('%064x',n),'fixture','audio/flac',8 FROM entries")
        .execute(&mut *transaction).await.unwrap();
    sqlx::query("INSERT INTO tracks(id,owner_id,media_asset_id,title,genre,codec,year,bitrate,sample_rate,bit_depth,duration_ms,added_at) SELECT id,'alice',id,'Common track','Jazz','FLAC',2026,1000,48000,24,1000,100 FROM media_assets")
        .execute(&mut *transaction).await.unwrap();
    sqlx::query(
        "INSERT INTO favorites(user_id,track_id) SELECT 'alice',id FROM tracks WHERE rowid<=3",
    )
    .execute(&mut *transaction)
    .await
    .unwrap();
    transaction.commit().await.unwrap();
    for sort in [TrackSort::Library, TrackSort::Title, TrackSort::Newest] {
        let filters = TrackFilters {
            genre: Some("jazz".to_owned()),
            codec: Some("flac".to_owned()),
            year_from: Some(2026),
            year_to: Some(2026),
            min_bitrate: Some(1000),
            min_sample_rate: Some(48000),
            min_bit_depth: Some(24),
            sort,
        };
        let first = fixture
            .media
            .list_tracks_filtered("alice", Some("common"), None, 2, true, &filters)
            .await
            .unwrap();
        assert_eq!(first.items.len(), 2);
        let second = fixture
            .media
            .list_tracks_filtered(
                "alice",
                Some("common"),
                first.next_cursor.as_deref(),
                2,
                true,
                &filters,
            )
            .await
            .unwrap();
        assert_eq!(second.items.len(), 1);
        assert!(second.next_cursor.is_none());
        let ids = first
            .items
            .into_iter()
            .chain(second.items)
            .map(|track| track.id)
            .collect::<HashSet<_>>();
        assert_eq!(ids.len(), 3);
        assert!(
            fixture
                .media
                .list_tracks_filtered("bob", Some("common"), None, 100, true, &filters)
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
}
