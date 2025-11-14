Feature: Sessions and check-ins
  Verify that the core auth + storage flows work end-to-end.

  Scenario: Registering and authenticating a user
    Given a fresh application state
    When I register a user "cutie" with email "cutie@example.com" and password "supersecret1"
    Then I can authenticate as "cutie" using password "supersecret1"

  Scenario: Creating check-ins for a user
    Given a fresh application state
    And a registered user "cutie" with email "cutie@example.com" and password "supersecret1"
    When I submit a check-in with mood 2 and high 3 and notes "Feeling cozy"
    Then the user has 1 stored check-ins
    And the latest stored check-in has mood 2 and high 3
