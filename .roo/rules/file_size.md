---
description: Guidelines for maintaining manageable file sizes and code organization
globs: **/*.rs,**/*.ts,**/*.js,**/*.py
alwaysApply: true
---

# File Size and Code Organization Guidelines

- **Maximum File Size:**
  - Keep files under 500 lines of code
  - Files approaching 300 lines should be considered for refactoring
  - Files over 500 lines MUST be refactored to improve maintainability

- **Code Organization Strategies:**
  - **Extract Modules:**
    - Move related functionality into separate modules
    - Use domain-driven boundaries for module separation
    ```rust
    // ✅ DO: Create separate modules for related functionality
    mod user_management;
    mod authentication;
    
    // ❌ DON'T: Keep growing a single file beyond 500 lines
    ```

  - **Extract Types:**
    - Move type definitions to separate files
    - Group related types together
    ```rust
    // ✅ DO: Move types to their own modules
    pub mod types {
        pub struct User { /* ... */ }
        pub struct Profile { /* ... */ }
    }
    
    // ❌ DON'T: Mix types, business logic, and utilities in one file
    ```

  - **Extract Functions:**
    - Break down large functions into smaller, focused ones
    - Aim for functions under 50 lines
    ```rust
    // ✅ DO: Extract helper functions
    fn process_data(data: &[u8]) -> Result<Output> {
        let validated = validate_data(data)?;
        let transformed = transform_data(validated)?;
        finalize_data(transformed)
    }
    
    // ❌ DON'T: Write monolithic functions
    ```

- **Module Structure:**
  - Use a clear hierarchical structure
  - Follow the principle of least knowledge between modules
  - Consider creating a `common` or `utils` module for shared functionality

- **Documentation:**
  - Add module-level documentation explaining the purpose and organization
  - Document the relationships between modules
  - Use diagrams for complex module hierarchies

- **Testing:**
  - Smaller files are easier to test
  - Extract complex logic to make it more testable
  - Consider test coverage when refactoring

- **Naming Conventions:**
  - Use clear, descriptive names for modules and files
  - Follow consistent naming patterns across the codebase
  - Name files according to their primary responsibility

- **Refactoring Triggers:**
  - File exceeds 300 lines
  - Function exceeds 50 lines
  - Module has more than 5-7 distinct responsibilities
  - Changes frequently affect multiple parts of the same file