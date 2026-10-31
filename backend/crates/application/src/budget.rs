//! Income, spending plans, spending records, and variance.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bigdecimal::BigDecimal;
use networth_domain::{
    BudgetAllocation, BudgetCategory, BudgetSpending, BudgetVariance, Currency, Money,
    NewBudgetIncome, NewBudgetSpending, SpendingPlan,
};
use uuid::Uuid;

use crate::ports::{BudgetRepository, RepositoryError};

/// Errors raised by budgeting use cases.
#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    /// Input failed validation.
    #[error("{0}")]
    Validation(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// A category with its percentage and planned amount.
#[derive(Debug, Clone)]
pub struct BudgetAllocationView {
    /// Category.
    pub category: BudgetCategory,
    /// Percentage of income.
    pub percentage: BigDecimal,
    /// Planned amount.
    pub planned: Money,
}

/// The complete budget view for one user/year.
#[derive(Debug, Clone)]
pub struct BudgetView {
    /// Year.
    pub year: i32,
    /// Currency.
    pub currency: Currency,
    /// Annual income, if configured.
    pub income: Option<Money>,
    /// Plan allocations.
    pub allocations: Vec<BudgetAllocationView>,
    /// Total allocated percentage.
    pub total_percentage: BigDecimal,
    /// Exactly 100% allocated.
    pub balanced: bool,
    /// More than 100% allocated.
    pub over_allocated: bool,
    /// Recorded spending.
    pub spending: Vec<BudgetSpending>,
    /// Planned versus actual.
    pub variance: Vec<BudgetVariance>,
}

/// Budgeting use cases.
pub struct BudgetService {
    repo: Arc<dyn BudgetRepository>,
}

impl BudgetService {
    /// Build the service.
    pub fn new(repo: Arc<dyn BudgetRepository>) -> Self {
        Self { repo }
    }

    /// Record or replace annual income.
    pub async fn set_income(
        &self,
        user_id: Uuid,
        year: i32,
        amount: Money,
    ) -> Result<(), BudgetError> {
        validate_year(year)?;
        if amount.is_negative() {
            return Err(BudgetError::Validation(
                "income must not be negative".to_string(),
            ));
        }
        self.repo
            .upsert_income(&NewBudgetIncome {
                user_id,
                year,
                amount,
            })
            .await?;
        Ok(())
    }

    /// Record or replace a spending plan.
    pub async fn set_plan(
        &self,
        user_id: Uuid,
        year: i32,
        currency: Currency,
        allocations: Vec<BudgetAllocation>,
    ) -> Result<SpendingPlan, BudgetError> {
        validate_year(year)?;
        let mut seen = HashSet::new();
        for allocation in &allocations {
            if !seen.insert(allocation.category) {
                return Err(BudgetError::Validation(format!(
                    "duplicate budget category: {}",
                    allocation.category
                )));
            }
            if allocation.percentage < 0 || allocation.percentage > 100 {
                return Err(BudgetError::Validation(
                    "each allocation must be between 0 and 100 percent".to_string(),
                ));
            }
        }
        Ok(self
            .repo
            .upsert_plan(user_id, year, &currency, &allocations)
            .await?)
    }

    /// Record or replace a category/month spending amount.
    pub async fn record_spending(
        &self,
        user_id: Uuid,
        year: i32,
        month: u32,
        category: BudgetCategory,
        amount: Money,
        note: Option<String>,
    ) -> Result<BudgetSpending, BudgetError> {
        validate_year(year)?;
        if !(1..=12).contains(&month) {
            return Err(BudgetError::Validation(
                "month must be between 1 and 12".to_string(),
            ));
        }
        if amount.is_negative() {
            return Err(BudgetError::Validation(
                "spending must not be negative".to_string(),
            ));
        }
        Ok(self
            .repo
            .upsert_spending(&NewBudgetSpending {
                user_id,
                year,
                month,
                category,
                amount,
                note: note
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
            })
            .await?)
    }

    /// Build the complete budget view for a user/year.
    pub async fn view(&self, user_id: Uuid, year: i32) -> Result<BudgetView, BudgetError> {
        validate_year(year)?;
        let income = self.repo.income(user_id, year).await?;
        let plan = self.repo.plan(user_id, year).await?;
        let spending = self.repo.list_spending(user_id, year).await?;
        let currency = plan
            .as_ref()
            .map(|plan| plan.currency.clone())
            .or_else(|| {
                income
                    .as_ref()
                    .map(|income| income.amount.currency().clone())
            })
            .unwrap_or_else(Currency::usd);

        let income_amount = income.as_ref().map(|income| income.amount.clone());
        let allocations: Vec<BudgetAllocationView> = plan
            .as_ref()
            .map(|plan| {
                plan.allocations
                    .iter()
                    .map(|allocation| BudgetAllocationView {
                        category: allocation.category,
                        percentage: allocation.percentage.clone(),
                        planned: planned_amount(
                            income_amount.as_ref(),
                            &allocation.percentage,
                            &currency,
                        ),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let total_percentage = plan
            .as_ref()
            .map(SpendingPlan::total_percentage)
            .unwrap_or_else(|| BigDecimal::from(0));
        let balanced = plan
            .as_ref()
            .map(SpendingPlan::is_balanced)
            .unwrap_or(false);
        let over_allocated = plan
            .as_ref()
            .map(SpendingPlan::is_over_allocated)
            .unwrap_or(false);

        let mut actual_by_category: HashMap<BudgetCategory, BigDecimal> = HashMap::new();
        for record in &spending {
            *actual_by_category
                .entry(record.category)
                .or_insert_with(|| BigDecimal::from(0)) += record.amount.amount();
        }

        let mut categories: Vec<BudgetCategory> = plan
            .as_ref()
            .map(|plan| {
                plan.allocations
                    .iter()
                    .map(|allocation| allocation.category)
                    .collect()
            })
            .unwrap_or_default();
        for category in actual_by_category.keys() {
            if !categories.contains(category) {
                categories.push(*category);
            }
        }

        let variance = categories
            .into_iter()
            .map(|category| {
                let percentage = plan
                    .as_ref()
                    .and_then(|plan| {
                        plan.allocations
                            .iter()
                            .find(|allocation| allocation.category == category)
                            .map(|allocation| allocation.percentage.clone())
                    })
                    .unwrap_or_else(|| BigDecimal::from(0));
                let planned = planned_amount(income_amount.as_ref(), &percentage, &currency);
                let actual = Money::new(
                    actual_by_category
                        .get(&category)
                        .cloned()
                        .unwrap_or_else(|| BigDecimal::from(0)),
                    currency.clone(),
                );
                let variance_amount = planned
                    .checked_sub(&actual)
                    .expect("budget amounts share a currency");
                let variance_percent = if planned.amount() == 0 {
                    None
                } else {
                    Some((variance_amount.amount() / planned.amount()) * BigDecimal::from(100))
                };
                BudgetVariance {
                    category,
                    planned,
                    actual,
                    variance_amount,
                    variance_percent,
                }
            })
            .collect();

        Ok(BudgetView {
            year,
            currency,
            income: income_amount,
            allocations,
            total_percentage,
            balanced,
            over_allocated,
            spending,
            variance,
        })
    }
}

fn planned_amount(income: Option<&Money>, percentage: &BigDecimal, currency: &Currency) -> Money {
    match income {
        Some(income) => Money::new(
            income.amount() * percentage / BigDecimal::from(100),
            currency.clone(),
        ),
        None => Money::zero(currency.clone()),
    }
}

fn validate_year(year: i32) -> Result<(), BudgetError> {
    if (1900..=2200).contains(&year) {
        Ok(())
    } else {
        Err(BudgetError::Validation(
            "year must be between 1900 and 2200".to_string(),
        ))
    }
}
