# TODO List

## Performance Optimizations

### Database Clone in regenerate_thumbnail.rs
- **Location**: `src/router/put/regenerate_thumbnail.rs:67`
- **Issue**: Using `database.clone()` to convert `DatabaseWithTag` to `Database` for `generate_dynamic_image()` call
- **Problem**: Unnecessary cloning of the entire `DatabaseWithTag` structure when only a reference conversion is needed
- **Solution**: Implement a more efficient conversion method or modify the function signature to accept `&DatabaseWithTag` and handle the conversion internally
- **Priority**: Medium
- **Status**: Pending