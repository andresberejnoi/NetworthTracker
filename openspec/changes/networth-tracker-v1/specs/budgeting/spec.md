# Spec Delta

## Purpose

Tracks income and lets the household apply a conscious-spending-style plan that allocates income
across categories, then compare actual spending against the plan.

## ADDED Requirements

### Requirement: Income recording
The system SHALL allow a user to record income amounts per user and period, in a currency, including
annual income used as the basis for a spending plan.

#### Scenario: Recording annual income
- **WHEN** a user enters their yearly income
- **THEN** the income is stored and available as the basis for allocating a spending plan

### Requirement: Spending plan allocation
The system SHALL let a user define a spending plan that allocates percentages of after-tax income
across categories, such as fixed costs, investments, savings, and discretionary spending.

#### Scenario: Defining a plan
- **WHEN** a user assigns percentages to categories that total the available income
- **THEN** the plan stores each category and its allocated amount derived from the income

#### Scenario: Over-allocation flagged
- **WHEN** the assigned percentages exceed the available income
- **THEN** the system flags the plan as over-allocated and prevents saving it as balanced

### Requirement: Category spending tracking
The system SHALL allow a user to record actual spending amounts per category and period so the plan
can be compared against reality.

#### Scenario: Recording actual spending
- **WHEN** a user records a spending amount against a category for a period
- **THEN** that amount is included in the category's total for that period

### Requirement: Plan versus actual variance
The system SHALL report the variance between planned and actual amounts per category and period.

#### Scenario: Variance reported
- **WHEN** a user has both a plan and recorded spending for a period
- **THEN** the system shows, per category, the planned amount, the actual amount, and the difference in amount and percentage

### Requirement: Budgeting is optional
The system SHALL function fully, including accounts, net worth, and history, when no budget or income
has been configured.

#### Scenario: No budget configured
- **WHEN** a user has never set up income or a spending plan
- **THEN** all other features remain available and the budgeting area simply appears empty
