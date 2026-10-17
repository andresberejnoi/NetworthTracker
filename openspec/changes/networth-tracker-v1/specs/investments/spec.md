# Spec Delta

## Purpose

Tracks the individual securities held inside investment accounts, including quantity, cost basis,
and classification, so the user can see what they own and how it is allocated.

## ADDED Requirements

### Requirement: Security holdings
The system SHALL record holdings of individual securities, such as stocks, ETFs, and mutual funds,
within an investment account, each with a symbol, a quantity, and a cost basis.

#### Scenario: Recording a stock position
- **WHEN** a user adds 10 shares of a ticker to a taxable brokerage account with a cost basis
- **THEN** the holding is stored and contributes to the account's value and allocation

### Requirement: Tax lots and cost basis
The system SHALL support multiple purchase lots per security, each with an acquisition date and its
own cost basis, and SHALL NOT require lots to be merged.

#### Scenario: Multiple lots for one security
- **WHEN** a user records two purchases of the same security on different dates
- **THEN** both lots are retained and the combined quantity and blended cost basis are reported

### Requirement: Asset class and sector classification
Each security SHALL be classifiable by asset class (for example equity, fixed income, real estate,
commodity, or cash) and, for equities, by sector, so holdings can be summarized across the
portfolio.

#### Scenario: Portfolio breakdown by class
- **WHEN** a user views the breakdown of an investment account
- **THEN** holdings are grouped by asset class and, where classified, by sector

### Requirement: Holdings valuation
The system SHALL compute a holding's current value from its quantity and the latest available price,
and SHALL fall back to the most recent stored price when no price source is reachable.

#### Scenario: Valuation from latest price
- **WHEN** a holding has a quantity and a current price is available
- **THEN** the holding's value equals quantity multiplied by price in the account's currency

#### Scenario: Offline valuation
- **WHEN** no price source is reachable for a holding
- **THEN** the holding is valued using the most recent stored price and is marked as stale

### Requirement: Unrealized gain and loss
The system SHALL compute unrealized gain or loss for each holding and for an investment account as a
whole, as current value minus cost basis.

#### Scenario: Gain reported
- **WHEN** a holding's current value exceeds its cost basis
- **THEN** the difference is reported as unrealized gain
