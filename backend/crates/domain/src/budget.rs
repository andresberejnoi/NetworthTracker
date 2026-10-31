//! Income, spending plans, category spending, and variance.

use std::fmt;
use std::str::FromStr;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::currency::Currency;
use crate::money::Money;

/// A spending category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetCategory {
    /// Rent, utilities, insurance, and other fixed costs.
    FixedCosts,
    /// Contributions to investments.
    Investments,
    /// Cash savings.
    Savings,
    /// Discretionary spending.
    Discretionary,
    /// Anything else.
    Other,
}

impl BudgetCategory {
    /// Every category.
    pub const ALL: [BudgetCategory; 5] = [
        BudgetCategory::FixedCosts,
        BudgetCategory::Investments,
        BudgetCategory::Savings,
        BudgetCategory::Discretionary,
        BudgetCategory::Other,
    ];

    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            BudgetCategory::FixedCosts => "fixed_costs",
            BudgetCategory::Investments => "investments",
            BudgetCategory::Savings => "savings",
            BudgetCategory::Discretionary => "discretionary",
            BudgetCategory::Other => "other",
        }
    }
}

impl fmt::Display for BudgetCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`BudgetCategory`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown budget category: {0}")]
pub struct BudgetCategoryParseError(pub String);

impl FromStr for BudgetCategory {
    type Err = BudgetCategoryParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fixed_costs" => Ok(BudgetCategory::FixedCosts),
            "investments" => Ok(BudgetCategory::Investments),
            "savings" => Ok(BudgetCategory::Savings),
            "discretionary" => Ok(BudgetCategory::Discretionary),
            "other" => Ok(BudgetCategory::Other),
            other => Err(BudgetCategoryParseError(other.to_string())),
        }
    }
}

/// Annual income for a user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetIncome {
    /// Identifier.
    pub id: Uuid,
    /// Owner.
    pub user_id: Uuid,
    /// Year.
    pub year: i32,
    /// Annual amount.
    pub amount: Money,
}

/// Fields for recording income.
#[derive(Debug, Clone)]
pub struct NewBudgetIncome {
    /// Owner.
    pub user_id: Uuid,
    /// Year.
    pub year: i32,
    /// Annual amount.
    pub amount: Money,
}

/// One percentage allocation in a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetAllocation {
    /// Category.
    pub category: BudgetCategory,
    /// Percentage of annual income.
    pub percentage: BigDecimal,
}

/// A spending plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendingPlan {
    /// Identifier.
    pub id: Uuid,
    /// Owner.
    pub user_id: Uuid,
    /// Year.
    pub year: i32,
    /// Currency.
    pub currency: Currency,
    /// Allocations.
    pub allocations: Vec<BudgetAllocation>,
}

impl SpendingPlan {
    /// Total allocated percentage.
    pub fn total_percentage(&self) -> BigDecimal {
        self.allocations
            .iter()
            .fold(BigDecimal::from(0), |total, allocation| {
                total + &allocation.percentage
            })
    }

    /// Whether the plan allocates exactly 100%.
    pub fn is_balanced(&self) -> bool {
        self.total_percentage() == 100
    }

    /// Whether the plan allocates more than 100%.
    pub fn is_over_allocated(&self) -> bool {
        self.total_percentage() > 100
    }
}

/// A spending record for one category/month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetSpending {
    /// Identifier.
    pub id: Uuid,
    /// Owner.
    pub user_id: Uuid,
    /// Year.
    pub year: i32,
    /// Month, 1–12.
    pub month: u32,
    /// Category.
    pub category: BudgetCategory,
    /// Amount spent.
    pub amount: Money,
    /// Optional note.
    pub note: Option<String>,
}

/// Fields for recording spending.
#[derive(Debug, Clone)]
pub struct NewBudgetSpending {
    /// Owner.
    pub user_id: Uuid,
    /// Year.
    pub year: i32,
    /// Month, 1–12.
    pub month: u32,
    /// Category.
    pub category: BudgetCategory,
    /// Amount spent.
    pub amount: Money,
    /// Optional note.
    pub note: Option<String>,
}

/// Planned versus actual for one category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetVariance {
    /// Category.
    pub category: BudgetCategory,
    /// Planned amount.
    pub planned: Money,
    /// Actual amount.
    pub actual: Money,
    /// Planned minus actual.
    pub variance_amount: Money,
    /// Difference as a percentage of planned, when planned is non-zero.
    pub variance_percent: Option<BigDecimal>,
}
