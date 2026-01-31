# Progress: calls-contract-tests

<!-- Steps are numbered. Substeps use parent.child notation. -->
<!-- Status markers: [ ] pending, [~] in_progress, [w] waiting, [x] done -->

## 1. [x] Set up foundation infrastructure
- [x] **1.1** Create contract-tests directory structure
- [x] **1.2** Create Dockerfile with PHP 8.3 and Composer
- [x] **1.3** Create docker-compose.yml with volume mounts
- [x] **1.4** Create composer.json with PHPUnit dependency
- [x] **1.5** Create phpunit.xml with test configuration
- [x] **1.6** Create config.php with path configuration
- [x] **1.7** Implement IndexGenerator class (runs scip-php binary)
- [x] **1.8** Implement bootstrap.php (loads config, generates index)
- [x] **1.9** Implement CallsData class (loads and indexes calls.json)
- [x] **1.10** Implement CallsContractTestCase base class
- [x] **1.11** Implement SmokeTest (must pass)
- [x] **1.12** Verify SmokeTest passes in Docker

## 2. [x] Implement Query API
- [x] **2.1** Implement ValueQuery class with filter methods
- [x] **2.2** Implement CallQuery class with filter methods
- [x] **2.3** Implement MethodScope helper class
- [x] **2.4** Add query factory methods to CallsContractTestCase

## 3. [x] Implement Assertion API
- [x] **3.1** Implement ReferenceConsistencyAssertion class
- [x] **3.2** Implement ChainIntegrityAssertion class
- [x] **3.3** Implement ChainVerificationResult class
- [x] **3.4** Implement ArgumentBindingAssertion class
- [x] **3.5** Implement DataIntegrityAssertion class
- [x] **3.6** Implement IntegrityReport class
- [x] **3.7** Add assertion factory methods to CallsContractTestCase

## 4. [x] Create documentation
- [x] **4.1** Create docs/reference/kloc-scip/contract-tests/README.md
- [x] **4.2** Create docs/reference/kloc-scip/contract-tests/framework-api.md
- [x] **4.3** Create docs/reference/kloc-scip/contract-tests/test-categories.md
- [x] **4.4** Create docs/reference/kloc-scip/contract-tests/writing-tests.md
- [x] **4.5** Create kloc-reference-project-php/contract-tests/CLAUDE.md

## 5. [x] Write real contract tests
- [x] **5.1** Create DataIntegrityTest with all integrity checks
- [x] **5.2** Create ParameterReferenceTest for OrderRepository::save()
- [x] **5.3** Create ParameterReferenceTest for OrderService::createOrder()
- [x] **5.4** Create ChainIntegrityTest for NotificationService chain
- [x] **5.5** Create ChainIntegrityTest for OrderService chain
- [x] **5.6** Create ArgumentBindingTest for EmailSender::send() calls
- [x] **5.7** Create ArgumentBindingTest for constructor calls

## 6. [x] Testing and validation
- [x] **6.1** Validate happy path tests pass
- [x] **6.2** Validate edge case handling
- [x] **6.3** Validate error handling and messages
- [x] **6.4** Validate Docker integration works
- [x] **6.5** Validate regression tests for known patterns

