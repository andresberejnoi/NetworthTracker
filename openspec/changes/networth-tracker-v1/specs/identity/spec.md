# Spec Delta

## Purpose

Manages users, households, per-user data scoping, and role-based access so several people can track
their net worth in one installation without seeing or editing each other's data.

## ADDED Requirements

### Requirement: Household with multiple user profiles
The system SHALL support a single household containing multiple user profiles so family members can
track their net worth independently within one installation.

#### Scenario: Adding a family member
- **WHEN** an owner creates a new user profile in the household
- **THEN** the household lists the new user and the new user can sign in with their own data scope

#### Scenario: Independent profiles
- **WHEN** two users each record balances in accounts they own
- **THEN** each user's default view reflects only their own accounts

### Requirement: Local authentication
The system SHALL authenticate users locally using credentials stored on the machine, and MUST allow
the owner to disable authentication for a single-user offline setup.

#### Scenario: Offline sign-in
- **WHEN** a user signs in while the machine has no network connection
- **THEN** authentication succeeds using locally stored credentials

#### Scenario: Authentication disabled
- **WHEN** the owner disables authentication for a single-user installation
- **THEN** the application is usable without signing in and no credentials are transmitted anywhere

### Requirement: Role-based access levels
The system SHALL assign each user a role that determines whether they may view and/or modify
household and account data, with at least an owner (full control), an editor (modify data), and a
viewer (read-only).

#### Scenario: Viewer cannot modify
- **WHEN** a user with the viewer role attempts to create or edit an account or balance
- **THEN** the system rejects the change and reports that the user lacks permission

#### Scenario: Owner manages members
- **WHEN** an owner changes another user's role
- **THEN** the other user's permissions change on their next authorized request

### Requirement: Per-user data scoping
The system SHALL allow an account to be owned by an individual user or shared with the household,
and SHALL restrict each user's view to accounts they own or that are shared with them.

#### Scenario: Shared account visible to members
- **WHEN** an owner marks a joint account as shared with the household
- **THEN** every household member with access can see that account in their views

#### Scenario: Private account restricted
- **WHEN** a user owns an account that is not shared
- **THEN** other household members cannot see or edit that account
