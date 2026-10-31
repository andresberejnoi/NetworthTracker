//! Securities, holdings, and valuation.

use std::collections::HashMap;
use std::sync::Arc;

use networth_domain::{
    allocate_fifo, Currency, Holding, LotError, Money, NewLot, NewSale, NewSecurity, PriceQuote,
    Sale, SaleMethod, Security,
};
use uuid::Uuid;

use crate::market_data::{PriceError, PriceService};
use crate::ports::{InvestmentRepository, RepositoryError};

/// Errors raised by investment use cases.
#[derive(Debug, thiserror::Error)]
pub enum InvestmentError {
    /// Input failed validation.
    #[error("{0}")]
    Validation(String),
    /// The resource does not exist.
    #[error("{0}")]
    NotFound(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// Price lookup failed.
    #[error(transparent)]
    Price(#[from] PriceError),
}

/// A holding with its current price and valuation.
#[derive(Debug, Clone)]
pub struct HoldingValuation {
    /// The position.
    pub holding: Holding,
    /// The latest quote, if one is available.
    pub price: Option<PriceQuote>,
    /// Quantity times price.
    pub market_value: Option<Money>,
    /// Market value minus cost basis.
    pub unrealized_gain: Option<Money>,
}

/// Securities and holdings use cases.
pub struct InvestmentsService {
    repo: Arc<dyn InvestmentRepository>,
    prices: Arc<PriceService>,
}

impl InvestmentsService {
    /// Build the service.
    pub fn new(repo: Arc<dyn InvestmentRepository>, prices: Arc<PriceService>) -> Self {
        Self { repo, prices }
    }

    /// Create a security.
    pub async fn create_security(&self, new: NewSecurity) -> Result<Security, InvestmentError> {
        let symbol = new.symbol.trim().to_ascii_uppercase();
        if symbol.is_empty() {
            return Err(InvestmentError::Validation(
                "symbol is required".to_string(),
            ));
        }
        if new.name.trim().is_empty() {
            return Err(InvestmentError::Validation("name is required".to_string()));
        }

        Ok(self
            .repo
            .create_security(&NewSecurity {
                symbol,
                name: new.name.trim().to_string(),
                asset_class: new.asset_class,
                sector: new.sector,
                currency: new.currency,
            })
            .await?)
    }

    /// List securities.
    pub async fn list_securities(&self) -> Result<Vec<Security>, InvestmentError> {
        Ok(self.repo.list_securities().await?)
    }

    /// Append purchase lots to a holding, creating it if needed.
    pub async fn append_lots(
        &self,
        account_id: Uuid,
        symbol: &str,
        lots: Vec<NewLot>,
    ) -> Result<Holding, InvestmentError> {
        if lots.is_empty() {
            return Err(InvestmentError::Validation(
                "a holding needs at least one lot".to_string(),
            ));
        }

        let symbol = symbol.trim().to_ascii_uppercase();
        let security = self
            .repo
            .find_security_by_symbol(&symbol)
            .await?
            .ok_or_else(|| {
                InvestmentError::NotFound(format!("security '{symbol}' not found; create it first"))
            })?;

        for lot in &lots {
            if lot.quantity.is_zero() {
                return Err(InvestmentError::Validation(
                    "lot quantity must be positive".to_string(),
                ));
            }
            if lot.cost_basis.is_negative() {
                return Err(InvestmentError::Validation(
                    "lot cost basis must not be negative".to_string(),
                ));
            }
            if lot.cost_basis.currency() != &security.currency {
                return Err(InvestmentError::Validation(format!(
                    "lot currency {} does not match security currency {}",
                    lot.cost_basis.currency(),
                    security.currency
                )));
            }
            if let Some(fees) = &lot.fees {
                if fees.currency() != &security.currency {
                    return Err(InvestmentError::Validation(format!(
                        "fee currency {} does not match security currency {}",
                        fees.currency(),
                        security.currency
                    )));
                }
                if fees.is_negative() {
                    return Err(InvestmentError::Validation(
                        "lot fees must not be negative".to_string(),
                    ));
                }
            }
        }

        Ok(self
            .repo
            .append_lots(account_id, security.id, &lots)
            .await?)
    }

    /// Record a sale and consume lots FIFO.
    pub async fn record_sale(
        &self,
        account_id: Uuid,
        symbol: &str,
        sale: NewSale,
    ) -> Result<Sale, InvestmentError> {
        if sale.method != SaleMethod::Fifo {
            return Err(InvestmentError::Validation(
                "only FIFO sales are supported".to_string(),
            ));
        }
        if sale.gross_proceeds.is_negative() || sale.fees.is_negative() {
            return Err(InvestmentError::Validation(
                "proceeds and fees must not be negative".to_string(),
            ));
        }

        let symbol = symbol.trim().to_ascii_uppercase();
        let security = self
            .repo
            .find_security_by_symbol(&symbol)
            .await?
            .ok_or_else(|| InvestmentError::NotFound(format!("security '{symbol}' not found")))?;

        if sale.currency != security.currency {
            return Err(InvestmentError::Validation(format!(
                "sale currency {} does not match security currency {}",
                sale.currency, security.currency
            )));
        }

        let holdings = self.repo.list_holdings(account_id).await?;
        let holding = holdings
            .into_iter()
            .find(|holding| holding.security.id == security.id)
            .ok_or_else(|| {
                InvestmentError::NotFound(format!("no {symbol} holding in account {account_id}"))
            })?;

        let allocations = allocate_fifo(&holding.lots, &sale).map_err(|error| match error {
            LotError::InsufficientQuantity {
                requested,
                available,
            } => InvestmentError::Validation(format!(
                "cannot sell {requested}: only {available} available"
            )),
            other => InvestmentError::Validation(other.to_string()),
        })?;

        Ok(self
            .repo
            .record_sale(holding.id, &sale, &allocations)
            .await?)
    }

    /// List recorded sales for an account, newest first.
    pub async fn list_sales(&self, account_id: Uuid) -> Result<Vec<Sale>, InvestmentError> {
        Ok(self.repo.list_sales(account_id).await?)
    }

    /// List holdings for an account with current valuations.
    pub async fn list_holdings(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<HoldingValuation>, InvestmentError> {
        let holdings = self.repo.list_holdings(account_id).await?;
        let mut quotes: HashMap<String, Option<PriceQuote>> = HashMap::new();
        let mut valuations = Vec::with_capacity(holdings.len());

        for holding in holdings {
            let quote = match quotes.get(&holding.security.symbol) {
                Some(cached) => cached.clone(),
                None => {
                    let quote = self.prices.quote(&holding.security.symbol).await?;
                    quotes.insert(holding.security.symbol.clone(), quote.clone());
                    quote
                }
            };

            let (market_value, unrealized_gain) = match &quote {
                Some(quote) => (
                    Some(holding.market_value(&quote.price)),
                    Some(holding.unrealized_gain(&quote.price)),
                ),
                None => (None, None),
            };

            valuations.push(HoldingValuation {
                holding,
                price: quote,
                market_value,
                unrealized_gain,
            });
        }

        Ok(valuations)
    }

    /// Delete a holding.
    pub async fn delete_holding(&self, holding_id: Uuid) -> Result<(), InvestmentError> {
        if self.repo.delete_holding(holding_id).await? {
            Ok(())
        } else {
            Err(InvestmentError::NotFound(format!(
                "holding not found: {holding_id}"
            )))
        }
    }

    /// Total holdings value per account, in the reporting currency.
    ///
    /// An account is included only when every one of its holdings has a price in
    /// the reporting currency; otherwise net worth falls back to the account's
    /// recorded balance.
    pub async fn account_values(
        &self,
        reporting: &Currency,
    ) -> Result<HashMap<Uuid, Money>, InvestmentError> {
        let holdings = self.repo.list_all_holdings().await?;
        let mut by_account: HashMap<Uuid, Vec<Holding>> = HashMap::new();
        for holding in holdings {
            by_account
                .entry(holding.account_id)
                .or_default()
                .push(holding);
        }

        let mut quotes: HashMap<String, Option<PriceQuote>> = HashMap::new();
        let mut values = HashMap::new();

        for (account_id, holdings) in by_account {
            let mut total: Option<Money> = None;
            let mut complete = true;

            for holding in &holdings {
                let quote = match quotes.get(&holding.security.symbol) {
                    Some(cached) => cached.clone(),
                    None => {
                        let quote = self.prices.quote(&holding.security.symbol).await?;
                        quotes.insert(holding.security.symbol.clone(), quote.clone());
                        quote
                    }
                };

                match quote {
                    Some(quote) if quote.price.currency() == reporting => {
                        let value = holding.market_value(&quote.price);
                        total = Some(match total {
                            Some(current) => current
                                .checked_add(&value)
                                .expect("holdings share the reporting currency"),
                            None => value,
                        });
                    }
                    _ => {
                        complete = false;
                        break;
                    }
                }
            }

            if complete {
                if let Some(total) = total {
                    values.insert(account_id, total);
                }
            }
        }

        Ok(values)
    }
}
