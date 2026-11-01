# Spec Delta

## Purpose

Provides security prices, cryptocurrency prices, and foreign-exchange rates used to value holdings,
with caching so historical snapshots stay reproducible and offline use remains possible.

## ADDED Requirements

### Requirement: Price retrieval through adapters
The system SHALL retrieve asset prices and foreign-exchange rates through pluggable provider
adapters, and SHALL NOT depend on any single provider.

#### Scenario: Configuring a provider
- **WHEN** an owner configures a price provider
- **THEN** the system uses that provider for subsequent price retrievals

#### Scenario: No provider configured
- **WHEN** no price provider is configured
- **THEN** the system relies on stored or manually entered prices without error

### Requirement: Cached prices with timestamps
The system SHALL cache every retrieved price with the asset identifier, the price value, its
currency, and the as-of timestamp of the price.

#### Scenario: Price stored on retrieval
- **WHEN** a price is fetched for an asset
- **THEN** it is stored with its currency and as-of timestamp and is available for later offline use

### Requirement: Offline fallback to last known price
WHEN no price provider is reachable, the system SHALL value assets using the most recent cached
price and SHALL mark the resulting value as stale.

#### Scenario: Stale valuation surfaced
- **WHEN** the system values an asset using a cached price because no provider is reachable
- **THEN** the value is marked stale so the user can distinguish it from a fresh quote

### Requirement: Snapshots record the price used
WHEN the system records a net worth snapshot, it SHALL store the price used for each valued asset so
history does not change retroactively when prices are later updated.

#### Scenario: History stability
- **WHEN** a price is later corrected or a newer price arrives
- **THEN** previously recorded snapshots still report the price that was used at the time

### Requirement: Manual valuation override
The system SHALL allow a user to set or override the value or price of an asset that has no market
quote, such as real estate or a private holding.

#### Scenario: Manual value for real estate
- **WHEN** a user enters a manual valuation for a property
- **THEN** that value is used for the property instead of any market price

### Requirement: Multi-currency conversion
The system SHALL convert amounts held in different currencies into a household reporting currency
using stored foreign-exchange rates, and SHALL record which rate was applied.

#### Scenario: Converting a foreign balance
- **WHEN** an account is denominated in a currency other than the reporting currency
- **THEN** its value is converted using a stored rate and the applied rate is retained

### Requirement: User-supplied provider credentials
The system SHALL require the individual user to supply their own API key or subscription for any
external price or FX provider, and MUST NOT bundle provider credentials or depend on a
NetworthTracker-operated relay service.

#### Scenario: Bring your own key
- **WHEN** a user enables a price provider
- **THEN** they supply their own API key or subscription, which is stored locally

#### Scenario: No central relay
- **WHEN** the application retrieves a price or rate
- **THEN** the request goes directly from the user's machine to the provider they configured

#### Scenario: Add-on inert until enabled
- **WHEN** no price add-on has been enabled
- **THEN** the system continues to value assets from stored or manually entered prices
