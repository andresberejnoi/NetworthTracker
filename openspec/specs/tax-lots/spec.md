# tax-lots Specification

## Purpose
Provides a shared, append-only tax-lot ledger for securities and cryptocurrency so
purchases, sales, transfers, cost basis, and realized gains stay auditable and
can be used for tax preparation. Lots are facts; holding-period labels and tax
jurisdiction rules are derived at query/report time.

## Requirements

### Requirement: Purchase lots are first-class and append-only
The system SHALL record each purchase as a lot with its acquisition date,
quantity, total cost basis, fees, currency, source, and an optional estimated
basis marker. Adding a lot MUST NOT delete or replace previously recorded lots.

#### Scenario: Appending a second purchase
- **WHEN** a user already has a lot of 10 shares acquired on 2026-01-01 and adds
  another lot of 5 shares acquired on 2026-02-01
- **THEN** the position shows both lots, a remaining quantity of 15, and a
  combined remaining cost basis

#### Scenario: Estimated basis
- **WHEN** a user records a historical lot with an approximate purchase price
- **THEN** the lot is stored with `basis_quality = estimated`, the UI/report shows
  an estimated-basis marker, and valuation/realized-gain calculations use the
  entered cost basis exactly like an actual-basis lot

### Requirement: Sales consume lots with FIFO
The system SHALL record a sale with its disposal date, quantity, gross proceeds,
fees, and currency, and SHALL consume lots in first-in-first-out order by
default. A sale that spans multiple lots SHALL create one disposal record per
consumed lot, storing the allocated proceeds, fees, and cost basis.

#### Scenario: FIFO across two lots
- **WHEN** a user sells 12 units after acquiring 10 units in January and 5 units
  in February
- **THEN** the sale consumes all 10 January units and 2 February units, and the
  remaining position is 3 February units

#### Scenario: Insufficient quantity
- **WHEN** a user tries to sell more units than the remaining position
- **THEN** the sale is rejected and no lots are changed

### Requirement: Realized gains and holding period are derived
The system SHALL derive each disposal's realized gain/loss and holding period
from the lot's acquisition date, the sale date, and a caller-supplied long-term
rule. The database MUST NOT store `short_term`/`long_term` or a holding-period
label; those are computed in the backend and included in API responses and
reports.

#### Scenario: Configurable long-term rule
- **WHEN** a report is requested with a long-term rule of more than one year
- **THEN** each disposal is classified using the exact calendar interval, not a
  hard-coded day count

#### Scenario: Term shown in the response
- **WHEN** a sale or realized-gains list is returned
- **THEN** each disposal includes its holding-period days and derived term

### Requirement: Lot and sale history is auditable
The system SHALL retain original lot quantities/cost bases and record disposals
rather than deleting lot history. Corrections MUST be represented as new
adjustment records or explicit edits with an audit trail, not silent replacement.

#### Scenario: Editing a position does not wipe history
- **WHEN** a user adds a lot to an existing position
- **THEN** existing lots and their remaining quantities are preserved

#### Scenario: Deleting a position with sales is refused
- **WHEN** a user tries to delete a position that already has recorded sales
- **THEN** the system refuses and keeps the tax history intact

### Requirement: Transfers preserve tax basis
The system SHALL move lots between wallets or accounts without realizing a gain,
preserving the original acquisition date and proportional cost basis.

#### Scenario: Wallet-to-wallet transfer
- **WHEN** a user transfers part of a lot from one wallet to another
- **THEN** the destination receives a lot slice with the same acquisition date
  and proportional cost basis, and the source's remaining quantity is reduced

### Requirement: Realized-gains export
The system SHALL produce a realized-gains report with acquisition date, disposal
date, quantity, proceeds, fees, cost basis, gain/loss, holding-period days,
derived term, and the estimated-basis marker for each disposal.

#### Scenario: Exporting a tax year
- **WHEN** a user requests realized gains for a tax year
- **THEN** the report contains one row per disposal in that year
