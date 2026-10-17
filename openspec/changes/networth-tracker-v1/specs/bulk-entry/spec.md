# Spec Delta

## Purpose

Lets a user enter many records of the same kind quickly in a spreadsheet-like grid instead of
repeating a one-record-at-a-time form workflow.

## ADDED Requirements

### Requirement: Spreadsheet-like grid entry
The system SHALL provide a spreadsheet-like grid for entering many records of one kind, such as
balances, holdings, or wallet addresses, in a single view with one row per record.

#### Scenario: Entering many records in one view
- **WHEN** a user opens bulk entry for cryptocurrency addresses
- **THEN** they can add row after row in the same grid without opening a separate form for each address

### Requirement: Keyboard-first entry
The system SHALL support adding and moving between rows and fields using the keyboard alone, so a
user can enter many records sequentially without using the mouse.

#### Scenario: Entering a run of records
- **WHEN** a user finishes a row and presses the key to add the next row
- **THEN** focus moves to the first field of a new empty row so entry can continue immediately

### Requirement: Clipboard paste of tabular data
The system SHALL accept pasted tabular data, for example copied from a spreadsheet, into the grid and
SHALL split it into rows and columns using the pasted delimiters.

#### Scenario: Pasting a block of rows
- **WHEN** a user pastes multiple tab-separated rows copied from a spreadsheet into the grid
- **THEN** the data is distributed across the grid's rows and columns ready for review

#### Scenario: Pasting twenty wallets
- **WHEN** a user pastes twenty address rows at once
- **THEN** all twenty appear as grid rows and none requires an individual form submission

### Requirement: Inline validation
The system SHALL validate each row inline and SHALL indicate which rows are invalid and why, while
allowing the user to correct them before saving.

#### Scenario: Invalid row flagged
- **WHEN** a pasted row contains an unparseable amount or an unknown value
- **THEN** that row is flagged with the reason and the remaining rows are unaffected

### Requirement: Single-action bulk save
The system SHALL save all valid rows in a single action and SHALL clearly report any rows that were
not saved.

#### Scenario: Saving the grid
- **WHEN** a user saves a grid containing only valid rows
- **THEN** all rows are persisted together and the grid reports success

#### Scenario: Partial failure reported
- **WHEN** a user saves a grid where some rows are invalid
- **THEN** the valid rows are saved or the batch is rejected as configured, and the user is told exactly which rows were skipped and why
