# Spec Delta

## Purpose

Makes the landing screen a real household net worth dashboard rather than a
backend-status page, while keeping advanced planning assumptions out of the
primary scan path.

## ADDED Requirements

### Requirement: Dashboard is the home route
The system SHALL make the net worth dashboard the default signed-in Home route.

#### Scenario: Signed-in landing
- **WHEN** a signed-in user opens the app root
- **THEN** the Home dashboard shows the household's current net worth summary
  rather than server metadata

#### Scenario: Dashboard unavailable
- **WHEN** dashboard data cannot be loaded
- **THEN** the Home route shows a clear error and a retry action while keeping
  navigation usable

### Requirement: Dashboard quick actions
The dashboard SHALL provide direct actions for the most common tasks: add a
balance, take a snapshot, and import or upload data, subject to the user's role.

#### Scenario: Add balance from home
- **WHEN** an owner/editor chooses Add balance on Home
- **THEN** a focused balance flow opens without navigating away from the
  dashboard context

#### Scenario: Read-only user
- **WHEN** a viewer opens Home
- **THEN** write actions are hidden or disabled and the dashboard remains
  readable

### Requirement: Secondary planning assumptions
The dashboard SHALL keep age, tax, penalty, selling-cost, and other planning
assumptions in a secondary panel or Settings flow, while the main summary shows
the resulting figures and period comparison.

#### Scenario: Adjust assumptions
- **WHEN** a user chooses to change planning assumptions
- **THEN** the controls open in a secondary drawer/panel and the dashboard
  updates without turning the main view into a settings form

### Requirement: Dashboard account context
The dashboard SHALL include a concise account summary or recent-activity area
that helps the user identify missing or stale balances without duplicating the
full Accounts management view.

#### Scenario: Stale data
- **WHEN** one or more accounts have stale or missing recent balances
- **THEN** the dashboard surfaces a concise indicator and a path to the affected
  account
