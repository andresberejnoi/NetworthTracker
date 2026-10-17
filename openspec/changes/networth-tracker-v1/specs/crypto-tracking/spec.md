# Spec Delta

## Purpose

Lets the user track cryptocurrency by public wallet address, deriving balances from chain data when
available, without ever holding private keys.

## ADDED Requirements

### Requirement: Wallet address tracking
The system SHALL allow a user to register public cryptocurrency addresses together with their chain
or network, and MUST NOT accept or store private keys, seed phrases, or any signing material.

#### Scenario: Registering an address
- **WHEN** a user enters a public address and selects its chain
- **THEN** the address is stored and associated with that chain as a crypto asset holding

#### Scenario: Rejecting secrets
- **WHEN** a user attempts to enter a value that is a private key or seed phrase
- **THEN** the system rejects the input and explains that only public addresses are accepted

### Requirement: On-chain balance derivation
The system SHALL derive an address's balance from a blockchain data source when one is available,
and SHALL associate derived balances with the address and chain they came from.

#### Scenario: Derived balance
- **WHEN** a registered address has a balance available from the configured chain data source
- **THEN** the derived balance is stored and used to value the crypto asset

### Requirement: High-precision crypto amounts
The system SHALL store cryptocurrency quantities with at least 18 significant decimal places and
MUST NOT round crypto quantities to fiat-style precision.

#### Scenario: Full-precision quantity preserved
- **WHEN** a user records a quantity such as 0.000000000000000001 of an asset
- **THEN** the stored and displayed value retains that precision

### Requirement: Offline manual crypto balances
WHEN no chain data source is reachable, the system SHALL allow the user to enter and edit crypto
balances manually, and MUST remain fully usable offline.

#### Scenario: Manual balance offline
- **WHEN** the user records a crypto balance while disconnected from any chain data source
- **THEN** the balance is stored and included in net worth like any other asset

### Requirement: Aggregation across addresses
The system SHALL aggregate multiple addresses or wallets holding the same asset into a single asset
position for net worth purposes while retaining per-address detail.

#### Scenario: Two wallets, one position
- **WHEN** a user has two addresses holding the same coin
- **THEN** net worth reflects the combined quantity and value while each address remains individually viewable
