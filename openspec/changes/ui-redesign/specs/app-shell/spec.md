# Spec Delta

## Purpose

Defines the responsive application shell and navigation model so the growing
number of views remains understandable and usable on desktop, tablet, and
mobile instead of overflowing into a row of competing tabs.

## ADDED Requirements

### Requirement: Responsive primary navigation
The system SHALL present a persistent sidebar on desktop and a mobile
navigation pattern with a bottom bar and a More sheet on small screens, without
page-level horizontal overflow.

#### Scenario: Desktop navigation
- **WHEN** the viewport is wide enough for desktop layout
- **THEN** a sidebar shows the primary destinations and optional/admin sections,
  and the active destination is visibly indicated

#### Scenario: Mobile navigation
- **WHEN** the viewport is narrow or the device is a phone
- **THEN** Home, Accounts, Portfolio, and More are reachable from a fixed bottom
  navigation bar, and secondary destinations open from the More sheet

#### Scenario: No tab spill
- **WHEN** the number of available destinations grows
- **THEN** navigation remains contained in the sidebar/More menu and never
  spills horizontally across the page

### Requirement: Primary and secondary information architecture
The system SHALL keep at most four primary destinations (Home, Accounts,
Portfolio, Data) and SHALL place optional and administrative destinations in a
secondary menu or Settings area.

#### Scenario: Optional features
- **WHEN** Budget or Aggregation is enabled
- **THEN** it is available from the secondary/sidebar menu rather than the
  primary navigation row

#### Scenario: Administrative features
- **WHEN** an owner opens Settings
- **THEN** Users, SSO, Aggregation, Backups, and About are grouped there, while
  non-owners do not see owner-only destinations

### Requirement: Route-based navigation
The system SHALL assign each primary and secondary destination a stable URL and
SHALL support browser back/forward, refresh, and deep links.

#### Scenario: Deep link
- **WHEN** a signed-in user opens an app URL directly
- **THEN** the corresponding view loads with its data and active navigation state

#### Scenario: Back navigation
- **WHEN** the user navigates between views and presses the browser Back button
- **THEN** the previous view is restored rather than resetting to an initial tab

### Requirement: Consistent page structure
Each view SHALL use a consistent page header with a title, short context, and at
most one primary action, with secondary actions visually subordinate.

#### Scenario: Page header
- **WHEN** a user opens a list or detail view
- **THEN** the title, purpose, and primary action are clear without scanning
  multiple competing forms

#### Scenario: Empty state
- **WHEN** a view has no records yet
- **THEN** it shows a short explanation and a direct action to create the first
  record

### Requirement: Progressive-disclosure forms
The system SHALL keep common fields visible and move advanced or rarely used
fields behind an expandable section, drawer, modal, or multi-step flow.

#### Scenario: Add account
- **WHEN** a user chooses to add an account
- **THEN** a focused form opens over the list instead of permanently occupying
  the page, and optional fields are secondary

#### Scenario: Advanced assumptions
- **WHEN** a user views the Home dashboard
- **THEN** age/tax/penalty/selling assumptions are available from a secondary
  control or Settings rather than occupying the main summary

### Requirement: Responsive tables and forms
Data tables and form controls SHALL fit the viewport using horizontal scrolling,
column prioritization, or mobile card rows, without causing the whole page to
overflow.

#### Scenario: Narrow table
- **WHEN** a table has more columns than fit on a phone
- **THEN** only the table region scrolls or columns collapse, while navigation
  and page actions remain usable

### Requirement: Accessible interaction
Navigation, drawers, menus, and forms SHALL be keyboard reachable, expose clear
focus states, use appropriate landmarks/labels, and close overlays with Escape.

#### Scenario: Keyboard navigation
- **WHEN** a keyboard-only user navigates the shell
- **THEN** focus order is logical and the active destination and open overlay are
  announced or visibly focused

#### Scenario: Mobile More sheet
- **WHEN** a user opens More and presses Escape or activates the close control
- **THEN** the sheet closes and focus returns to the triggering control
