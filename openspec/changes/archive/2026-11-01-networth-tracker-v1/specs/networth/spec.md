# Spec Delta

## Purpose

Computes net worth in several useful forms, total, liquid, and accessible, and persists point-in-time
history, applying configurable reductions for taxes, penalties, and liquidation costs.

## ADDED Requirements

### Requirement: Total net worth
The system SHALL compute total net worth at a given point in time as the sum of asset account values
minus liability balances, expressed in the household reporting currency.

#### Scenario: Total net worth
- **WHEN** the household has assets of 500,000 and liabilities of 200,000 on a date
- **THEN** total net worth for that date is reported as 300,000

### Requirement: Liquid net worth
The system SHALL compute a liquid net worth from assets that can be quickly converted to cash,
including cash, deposit accounts, taxable brokerage, and retirement accounts, and SHALL exclude
illiquid assets such as real estate.

#### Scenario: Retirement accounts are liquid
- **WHEN** the household holds a retirement account
- **THEN** its value is included in liquid net worth

#### Scenario: Illiquid assets excluded
- **WHEN** the household holds real estate in addition to liquid assets
- **THEN** liquid net worth excludes the property while total net worth includes it

#### Scenario: Debt secured by an excluded asset
- **WHEN** a mortgage is secured by a property that is excluded from liquid net worth
- **THEN** the mortgage does not reduce liquid net worth

### Requirement: Liquidity is cash on hand
The system SHALL report a liquidity figure equal to cash and cash-like assets that are available
immediately, such as checking, savings, and cash, and SHALL NOT include assets that must first be
sold.

#### Scenario: Only immediate cash counts
- **WHEN** the household holds cash, a brokerage account, and a retirement account
- **THEN** liquidity includes only the cash while liquid net worth includes all three

#### Scenario: Immediately due debt reduces liquidity
- **WHEN** a liability is immediately due rather than secured by an excluded asset
- **THEN** it reduces liquidity

### Requirement: Accessible net worth
The system SHALL compute an accessible net worth by applying reductions to each asset for taxes,
early-withdrawal penalties, and selling or closing costs, so restricted assets are reported at their
net realizable value.

#### Scenario: Retirement account discounted
- **WHEN** a retirement account is included in accessible net worth and the applicable tax rate and early-withdrawal penalty are configured
- **THEN** the account contributes its value minus the tax and penalty amounts

#### Scenario: Real estate discounted
- **WHEN** a property is included in accessible net worth and selling costs and taxes are configured
- **THEN** the property contributes its market value minus those costs rather than its full value

### Requirement: Configurable redemption rules
The system SHALL let a user configure the reductions used by accessible net worth, including a tax
rate, an early-withdrawal penalty that may depend on the owner's age, and selling or closing costs
expressed as either a percentage or a fixed amount.

#### Scenario: Age-dependent penalty
- **WHEN** a user is below the configured retirement age and holds a retirement account
- **THEN** the accessible value of that account applies the early-withdrawal penalty

#### Scenario: Adjusting a rule
- **WHEN** a user changes the marginal tax rate used for a retirement account type
- **THEN** accessible net worth recomputes using the new rate

#### Scenario: Composited selling costs
- **WHEN** a user models a property sale with a percentage commission and a fixed closing cost
- **THEN** both the percentage and the fixed components are subtracted from the market value

### Requirement: What-if scenario modeling
The system SHALL let a user model accessible net worth under alternative assumptions, such as a
different tax rate or selling-cost percentage, and MUST NOT change stored data until the user
chooses to apply the modeled assumptions.

#### Scenario: Modeling without saving
- **WHEN** a user adjusts scenario assumptions and views the result
- **THEN** projected accessible values are shown without persisting the change

#### Scenario: Applying a modeled scenario
- **WHEN** a user decides to keep a modeled scenario
- **THEN** the chosen assumptions are saved as the active configuration

### Requirement: Aggregation scope
The system SHALL compute net worth for the whole household and for each individual user, without
double-counting accounts shared between users.

#### Scenario: Household and per-user totals
- **WHEN** a joint account is shared between two users
- **THEN** the household total counts it once and each user's personal view includes it as shared

### Requirement: Net worth history snapshots
The system SHALL persist dated net worth snapshots capturing total, liquid, and accessible values so
the dashboard can chart change over time.

#### Scenario: Snapshot on demand
- **WHEN** a user takes a net worth snapshot on a date
- **THEN** the total, liquid, and accessible values for that date are stored for later charting

### Requirement: Snapshot reproducibility
A stored snapshot SHALL remain stable over time, so later changes to prices, balances, or
redemption rules MUST NOT alter the already-recorded values of that snapshot.

#### Scenario: Later edits do not rewrite history
- **WHEN** a balance from a past period is edited after a snapshot was taken
- **THEN** the existing snapshot keeps its recorded values until the user explicitly recomputes it
