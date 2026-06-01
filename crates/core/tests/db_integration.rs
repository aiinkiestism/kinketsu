//! End-to-end DB integration tests.
//!
//! Each test spins up an isolated in-memory SQLite pool, runs the bundled
//! migrations, and exercises one repository module top-to-bottom.

use chrono::{Duration, NaiveDate, Utc};
use kinketsu_core::currency::ExchangeRate;
use kinketsu_core::db;
use kinketsu_core::models::{
    BillingCycle, Category, DetectionEvent, DetectionSource, DetectionStatus, NewCategory,
    NewPaymentMethod, NewSubscription, PaymentMethodKind, Subscription, SubscriptionStatus,
};
use sqlx::SqlitePool;
use uuid::Uuid;

async fn setup_pool() -> SqlitePool {
    let pool = db::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    db::migrate(&pool).await.expect("migrate schema");
    pool
}

fn fresh_subscription(name: &str) -> Subscription {
    let now = Utc::now();
    Subscription {
        id: Uuid::now_v7(),
        name: name.into(),
        service_icon: None,
        plan: None,
        amount_minor: 1980,
        currency: "JPY".into(),
        billing_cycle: BillingCycle::Monthly,
        next_billing_date: Some(NaiveDate::from_ymd_opt(2026, 7, 15).unwrap()),
        started_at: None,
        payment_method_id: None,
        category_id: None,
        status: SubscriptionStatus::Active,
        notes: None,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn subscriptions_round_trip() {
    let pool = setup_pool().await;
    let sub = fresh_subscription("Netflix");
    let id = sub.id;

    db::subscriptions::insert(&pool, &sub).await.unwrap();

    let all = db::subscriptions::list(&pool).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "Netflix");

    let fetched = db::subscriptions::get(&pool, id).await.unwrap().unwrap();
    assert_eq!(fetched.billing_cycle, BillingCycle::Monthly);
    assert_eq!(fetched.status, SubscriptionStatus::Active);

    let mut updated = fetched;
    updated.name = "Spotify".into();
    updated.status = SubscriptionStatus::Paused;
    updated.updated_at = Utc::now();
    db::subscriptions::update(&pool, &updated).await.unwrap();
    let after = db::subscriptions::get(&pool, id).await.unwrap().unwrap();
    assert_eq!(after.name, "Spotify");
    assert_eq!(after.status, SubscriptionStatus::Paused);

    db::subscriptions::delete(&pool, id).await.unwrap();
    assert!(db::subscriptions::get(&pool, id).await.unwrap().is_none());
    assert!(db::subscriptions::list(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn subscriptions_list_due_within_filters_active_and_window() {
    let pool = setup_pool().await;
    let today = Utc::now().date_naive();

    let mut soon = fresh_subscription("Within window");
    soon.next_billing_date = Some(today + Duration::days(3));

    let mut later = fresh_subscription("Outside window");
    later.next_billing_date = Some(today + Duration::days(30));

    let mut cancelled = fresh_subscription("Cancelled");
    cancelled.next_billing_date = Some(today + Duration::days(2));
    cancelled.status = SubscriptionStatus::Cancelled;

    let mut undated = fresh_subscription("No date");
    undated.next_billing_date = None;

    for s in [&soon, &later, &cancelled, &undated] {
        db::subscriptions::insert(&pool, s).await.unwrap();
    }

    let due = db::subscriptions::list_due_within(&pool, 7).await.unwrap();
    let names: Vec<_> = due.into_iter().map(|s| s.name).collect();
    assert_eq!(names, vec!["Within window".to_string()]);
}

#[tokio::test]
async fn payment_methods_round_trip() {
    let pool = setup_pool().await;

    let pm = NewPaymentMethod {
        name: "Visa ****1234".into(),
        kind: PaymentMethodKind::CreditCard,
        last4: Some("1234".into()),
        color: None,
        icon: None,
    }
    .into_payment_method();
    let id = pm.id;
    db::payment_methods::insert(&pool, &pm).await.unwrap();

    let all = db::payment_methods::list(&pool).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].kind, PaymentMethodKind::CreditCard);

    let mut updated = db::payment_methods::get(&pool, id).await.unwrap().unwrap();
    updated.kind = PaymentMethodKind::Wallet;
    updated.updated_at = Utc::now();
    db::payment_methods::update(&pool, &updated).await.unwrap();
    let after = db::payment_methods::get(&pool, id).await.unwrap().unwrap();
    assert_eq!(after.kind, PaymentMethodKind::Wallet);

    db::payment_methods::delete(&pool, id).await.unwrap();
    assert!(db::payment_methods::list(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn categories_round_trip_with_unique_name() {
    let pool = setup_pool().await;
    let cat = NewCategory {
        name: "Streaming".into(),
        icon: None,
        color: None,
    }
    .into_category();
    db::categories::insert(&pool, &cat).await.unwrap();

    // Duplicate name should violate UNIQUE constraint
    let dup = NewCategory {
        name: "Streaming".into(),
        icon: None,
        color: None,
    }
    .into_category();
    assert!(
        db::categories::insert(&pool, &dup).await.is_err(),
        "expected UNIQUE violation"
    );

    let mut updated: Category = db::categories::get(&pool, cat.id).await.unwrap().unwrap();
    updated.name = "Music & Streaming".into();
    updated.updated_at = Utc::now();
    db::categories::update(&pool, &updated).await.unwrap();
    let after = db::categories::get(&pool, cat.id).await.unwrap().unwrap();
    assert_eq!(after.name, "Music & Streaming");
}

#[tokio::test]
async fn settings_get_set_delete_with_json_roundtrip() {
    let pool = setup_pool().await;

    let missing: Option<String> = db::settings::get(&pool, "absent").await.unwrap();
    assert!(missing.is_none());

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Sample {
        name: String,
        count: u32,
    }
    let s = Sample {
        name: "kinketsu".into(),
        count: 42,
    };
    db::settings::set(&pool, "sample", &s).await.unwrap();

    let loaded: Sample = db::settings::get(&pool, "sample").await.unwrap().unwrap();
    assert_eq!(loaded, s);

    // Upsert via set
    let s2 = Sample {
        name: "kinketsu".into(),
        count: 99,
    };
    db::settings::set(&pool, "sample", &s2).await.unwrap();
    let loaded2: Sample = db::settings::get(&pool, "sample").await.unwrap().unwrap();
    assert_eq!(loaded2.count, 99);

    db::settings::delete(&pool, "sample").await.unwrap();
    let after: Option<Sample> = db::settings::get(&pool, "sample").await.unwrap();
    assert!(after.is_none());
}

#[tokio::test]
async fn exchange_rates_upsert_and_latest_only() {
    let pool = setup_pool().await;
    let earlier = Utc::now() - Duration::days(1);
    let now = Utc::now();

    db::exchange_rates::upsert(
        &pool,
        &ExchangeRate {
            base: "JPY".into(),
            quote: "USD".into(),
            rate: 0.0064,
            fetched_at: earlier,
        },
    )
    .await
    .unwrap();
    db::exchange_rates::upsert(
        &pool,
        &ExchangeRate {
            base: "JPY".into(),
            quote: "EUR".into(),
            rate: 0.0059,
            fetched_at: earlier,
        },
    )
    .await
    .unwrap();
    db::exchange_rates::upsert(
        &pool,
        &ExchangeRate {
            base: "JPY".into(),
            quote: "USD".into(),
            rate: 0.0065,
            fetched_at: now,
        },
    )
    .await
    .unwrap();
    db::exchange_rates::upsert(
        &pool,
        &ExchangeRate {
            base: "JPY".into(),
            quote: "EUR".into(),
            rate: 0.0060,
            fetched_at: now,
        },
    )
    .await
    .unwrap();

    let latest = db::exchange_rates::list_latest_for_base(&pool, "JPY")
        .await
        .unwrap();
    assert_eq!(latest.len(), 2);
    let usd = latest.iter().find(|r| r.quote == "USD").unwrap();
    let eur = latest.iter().find(|r| r.quote == "EUR").unwrap();
    assert!((usd.rate - 0.0065).abs() < 1e-9);
    assert!((eur.rate - 0.0060).abs() < 1e-9);

    let empty = db::exchange_rates::list_latest_for_base(&pool, "USD")
        .await
        .unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
async fn detection_events_crud_and_status_transition() {
    let pool = setup_pool().await;
    let now = Utc::now();

    let ev = DetectionEvent {
        id: Uuid::now_v7(),
        source: DetectionSource::Gmail,
        source_ref: Some("gmail-123".into()),
        raw_summary: Some("Netflix renewal".into()),
        sender: Some("[email protected]".into()),
        parsed_payload: serde_json::json!({ "service_name": "Netflix" }),
        confidence: 0.85,
        status: DetectionStatus::Pending,
        matched_subscription_id: None,
        reviewed_at: None,
        created_at: now,
    };
    db::detection_events::insert(&pool, &ev).await.unwrap();

    let fetched = db::detection_events::get(&pool, ev.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.status, DetectionStatus::Pending);
    assert_eq!(
        fetched.parsed_payload.get("service_name").unwrap(),
        "Netflix"
    );

    let pending = db::detection_events::list_by_status(&pool, DetectionStatus::Pending)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);

    db::detection_events::update_status(&pool, ev.id, DetectionStatus::Rejected, None)
        .await
        .unwrap();
    let rejected = db::detection_events::get(&pool, ev.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(rejected.status, DetectionStatus::Rejected);
    assert!(rejected.reviewed_at.is_some());

    let by_ref =
        db::detection_events::find_by_source_ref(&pool, DetectionSource::Gmail, "gmail-123")
            .await
            .unwrap();
    assert!(by_ref.is_some());

    let missing_ref =
        db::detection_events::find_by_source_ref(&pool, DetectionSource::Gmail, "nope")
            .await
            .unwrap();
    assert!(missing_ref.is_none());
}

#[tokio::test]
async fn subscription_payment_method_fk_set_null_on_delete() {
    let pool = setup_pool().await;
    let pm = NewPaymentMethod {
        name: "Card".into(),
        kind: PaymentMethodKind::CreditCard,
        last4: None,
        color: None,
        icon: None,
    }
    .into_payment_method();
    db::payment_methods::insert(&pool, &pm).await.unwrap();

    let mut sub = NewSubscription {
        name: "Netflix".into(),
        service_icon: None,
        plan: None,
        amount_minor: 1500,
        currency: "JPY".into(),
        billing_cycle: BillingCycle::Monthly,
        next_billing_date: None,
        started_at: None,
        payment_method_id: Some(pm.id),
        category_id: None,
        status: None,
        notes: None,
    }
    .into_subscription();
    sub.updated_at = Utc::now();
    db::subscriptions::insert(&pool, &sub).await.unwrap();

    db::payment_methods::delete(&pool, pm.id).await.unwrap();

    let after = db::subscriptions::get(&pool, sub.id)
        .await
        .unwrap()
        .unwrap();
    assert!(after.payment_method_id.is_none());
}
