use amanclaw_core::context_engine::format_learned_corrections;
use amanclaw_memory::knowledge_store::*;
use sqlx::Row;
use sqlx::sqlite::SqlitePoolOptions;

async fn make_store() -> KnowledgeStore {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(amanclaw_memory::schema::INIT_SQL)
        .execute(&pool)
        .await
        .unwrap();
    let store = KnowledgeStore::new(pool);
    store.init().await.unwrap();
    store
}

/// Full lifecycle: upsert → log event → query → format → record hit → retract → verify empty.
#[tokio::test]
async fn test_full_rle_flow() {
    let store = make_store().await;

    // 1. Upsert a correction (user corrects "Asr 4:30" to "4:15")
    let correction = DetectedCorrection {
        trigger: "Asr prayer time".to_string(),
        wrong_response: Some("Asr is at 4:30 PM".to_string()),
        correct_response: "Asr is at 4:15 PM".to_string(),
        topic: Some("solat".to_string()),
        confidence: 0.90,
        signal_type: "explicit_correction".to_string(),
    };

    let rule_id = store
        .upsert_rule(&correction, Some("user-1"), None, "user")
        .await
        .unwrap();
    assert!(rule_id > 0);

    // Promote to active so it appears in queries
    let promoted = store.promote_candidates(0.70).await.unwrap();
    assert_eq!(promoted, 1);

    // 2. Log a correction event
    let event = CorrectionEvent {
        rule_id: Some(rule_id),
        user_id: "user-1".to_string(),
        platform: Some("telegram".to_string()),
        signal_type: "explicit_correction".to_string(),
        signal_confidence: 0.90,
        source_user_msg: Some("No, Asr is at 4:15 not 4:30".to_string()),
        source_bot_msg: Some("Asr is at 4:30 PM".to_string()),
    };
    store.log_event(&event).await.unwrap();

    // Verify event was stored
    let row = sqlx::query("SELECT COUNT(*) as count FROM correction_events")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    let event_count: i64 = row.get("count");
    assert_eq!(event_count, 1);

    // 3. Query — should find the correction
    let query = CorrectionQuery {
        user_message: "What time is Asr prayer today".to_string(),
        user_id: "user-1".to_string(),
        community_id: None,
        topic: Some("solat".to_string()),
    };
    let matches = store.query(&query).await.unwrap();
    assert!(
        !matches.is_empty(),
        "query should return at least one match"
    );
    assert_eq!(matches[0].rule.correct_response, "Asr is at 4:15 PM");
    assert_eq!(matches[0].rule.layer, "user");

    // 4. Format for LLM context — should contain the correction text and "Learned knowledge"
    let formatted = format_learned_corrections(&matches);
    assert!(
        formatted.contains("Learned knowledge"),
        "formatted output should contain 'Learned knowledge'"
    );
    assert!(
        formatted.contains("Asr is at 4:15 PM"),
        "formatted output should contain correction text"
    );

    // 5. Record a hit — verify hit_count increments
    store.record_hit(rule_id).await.unwrap();
    let rules = store.get_user_rules("user-1").await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].hit_count, 1);

    store.record_hit(rule_id).await.unwrap();
    let rules = store.get_user_rules("user-1").await.unwrap();
    assert_eq!(rules[0].hit_count, 2);

    // 6. Retract all user rules
    let retracted = store.retract_all_user_rules("user-1").await.unwrap();
    assert_eq!(retracted, 1);

    // 7. Query again — should return empty
    let matches_after = store.query(&query).await.unwrap();
    assert!(
        matches_after.is_empty(),
        "query should return empty after retraction"
    );
}

/// Community-level knowledge propagates to new users in the same community.
#[tokio::test]
async fn test_community_knowledge_propagates() {
    let store = make_store().await;

    // 1. Admin upserts a community-level correction (Shafi'i method) for community "masjid-abc"
    let correction = DetectedCorrection {
        trigger: "calculation method for prayer times".to_string(),
        wrong_response: Some("Using Hanafi method".to_string()),
        correct_response: "This community uses the Shafi'i method for prayer time calculation"
            .to_string(),
        topic: Some("solat".to_string()),
        confidence: 0.92,
        signal_type: "admin_correction".to_string(),
    };

    store
        .upsert_rule(&correction, None, Some("masjid-abc"), "community")
        .await
        .unwrap();

    // Promote to active
    store.promote_candidates(0.70).await.unwrap();

    // 2. New user in same community queries — should find the community correction
    let query = CorrectionQuery {
        user_message: "What calculation method do we use for prayer times".to_string(),
        user_id: "new-user-xyz".to_string(),
        community_id: Some("masjid-abc".to_string()),
        topic: Some("solat".to_string()),
    };

    let matches = store.query(&query).await.unwrap();
    assert!(
        !matches.is_empty(),
        "new user should see community corrections"
    );

    // 3. Verify the match layer is "community"
    assert_eq!(matches[0].rule.layer, "community");
    assert!(matches[0].rule.correct_response.contains("Shafi'i method"));
}

/// Candidate promotion flow: low confidence → excluded → bump confidence → promote → active.
#[tokio::test]
async fn test_candidate_promotion_flow() {
    let store = make_store().await;

    // 1. Insert a low-confidence correction (0.50) — becomes candidate
    let correction = DetectedCorrection {
        trigger: "mosque parking info".to_string(),
        wrong_response: None,
        correct_response: "Parking is available behind the mosque".to_string(),
        topic: Some("masjid".to_string()),
        confidence: 0.50,
        signal_type: "implicit_feedback".to_string(),
    };

    let rule_id = store
        .upsert_rule(&correction, Some("user-2"), None, "user")
        .await
        .unwrap();

    // 2. Verify get_user_rules returns empty (candidates excluded — only 'active' returned)
    let rules = store.get_user_rules("user-2").await.unwrap();
    assert!(
        rules.is_empty(),
        "candidates should not appear in get_user_rules"
    );

    // 3. Manually bump confidence to 0.75 in DB
    sqlx::query("UPDATE correction_rules SET confidence = 0.75 WHERE id = ?")
        .bind(rule_id)
        .execute(&store.pool)
        .await
        .unwrap();

    // 4. Call promote_candidates(0.70) — should promote 1
    let promoted = store.promote_candidates(0.70).await.unwrap();
    assert_eq!(promoted, 1, "should promote exactly 1 candidate");

    // 5. Verify get_user_rules now returns the rule as "active"
    let rules = store.get_user_rules("user-2").await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].status, "active");
    assert_eq!(
        rules[0].correct_response,
        "Parking is available behind the mosque"
    );
}
