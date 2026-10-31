//! API-level tests for income, spending plans, spending, and variance.

mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn empty_budget_is_valid() {
    let app = common::router();
    let (status, budget) = common::get(&app, "/api/v1/budget?year=2026").await;
    assert_eq!(status, StatusCode::OK, "budget: {budget}");
    assert_eq!(budget["year"], 2026);
    assert!(budget["income"].is_null());
    assert_eq!(
        budget["allocations"].as_array().expect("allocations").len(),
        0
    );
    assert_eq!(budget["balanced"], false);
    assert_eq!(budget["over_allocated"], false);
}

#[tokio::test]
async fn records_income_plan_spending_and_variance() {
    let app = common::router();

    let (status, _) = common::post_json(
        &app,
        "/api/v1/budget/income",
        json!({ "year": 2026, "amount": "120000.00" }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = common::post_json(
        &app,
        "/api/v1/budget/plan",
        json!({
            "year": 2026,
            "allocations": [
                { "category": "fixed_costs", "percentage": "50" },
                { "category": "investments", "percentage": "20" },
                { "category": "savings", "percentage": "10" },
                { "category": "discretionary", "percentage": "15" },
                { "category": "other", "percentage": "5" }
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, budget) = common::get(&app, "/api/v1/budget?year=2026").await;
    assert_eq!(status, StatusCode::OK, "budget: {budget}");
    assert_eq!(budget["income"], "120000.00");
    assert_eq!(budget["balanced"], true);
    assert_eq!(budget["over_allocated"], false);
    assert_eq!(budget["total_percentage"], "100");
    let fixed = budget["allocations"]
        .as_array()
        .expect("allocations")
        .iter()
        .find(|row| row["category"] == "fixed_costs")
        .expect("fixed costs");
    assert_eq!(fixed["planned"], "60000.00");

    for (category, amount) in [("fixed_costs", "1000.00"), ("discretionary", "500.00")] {
        let (status, _) = common::post_json(
            &app,
            "/api/v1/budget/spending",
            json!({
                "year": 2026,
                "month": 1,
                "category": category,
                "amount": amount
            }),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    let (_, budget) = common::get(&app, "/api/v1/budget?year=2026").await;
    let fixed = budget["variance"]
        .as_array()
        .expect("variance")
        .iter()
        .find(|row| row["category"] == "fixed_costs")
        .expect("fixed variance");
    assert_eq!(fixed["planned"], "60000.00");
    assert_eq!(fixed["actual"], "1000.00");
    assert_eq!(fixed["variance_amount"], "59000.00");
    assert!(fixed["variance_percent"].as_str().is_some());
}

#[tokio::test]
async fn over_allocated_plan_is_flagged() {
    let app = common::router();
    let (status, _) = common::post_json(
        &app,
        "/api/v1/budget/plan",
        json!({
            "year": 2026,
            "allocations": [
                { "category": "fixed_costs", "percentage": "60" },
                { "category": "discretionary", "percentage": "50" }
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, budget) = common::get(&app, "/api/v1/budget?year=2026").await;
    assert_eq!(budget["balanced"], false);
    assert_eq!(budget["over_allocated"], true);
    assert_eq!(budget["total_percentage"], "110");
}

#[tokio::test]
async fn rejects_invalid_percentages_and_months() {
    let app = common::router();
    let (status, _) = common::post_json(
        &app,
        "/api/v1/budget/plan",
        json!({
            "year": 2026,
            "allocations": [{ "category": "fixed_costs", "percentage": "101" }]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = common::post_json(
        &app,
        "/api/v1/budget/spending",
        json!({
            "year": 2026,
            "month": 13,
            "category": "fixed_costs",
            "amount": "10.00"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
