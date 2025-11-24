# TODO List

## Performance Optimizations

### Database Clone in regenerate_thumbnail.rs
- **Location**: `src/router/put/regenerate_thumbnail.rs:67`
- **Issue**: Using `database.clone()` to convert `DatabaseWithTag` to `Database` for `generate_dynamic_image()` call
- **Problem**: Unnecessary cloning of the entire `DatabaseWithTag` structure when only a reference conversion is needed
- **Solution**: Implement a more efficient conversion method or modify the function signature to accept `&DatabaseWithTag` and handle the conversion internally
- **Priority**: Medium
- **Status**: Pending

### Database Connection in compute_timestamp_ms
- **Location**: `src/public/structure/database_struct/database/generate_timestamp.rs:16`
- **Issue**: Each call to `compute_timestamp_ms` acquires a new database connection via `TREE.get_connection()`
- **Problem**: Potential performance overhead from repeated connection acquisition, especially if called frequently
- **Solution**: Consider passing the connection as a parameter, using a connection pool, or caching the alias data at a higher level to avoid repeated queries
- **Priority**: Medium
- **Status**: Pending

### Alias Handling Refactor in deduplicate.rs
- **Location**: `src/tasks/actor/deduplicate.rs`
- **Issue**: Originally moved alias from new database to existing database using `mem::take(&mut database.alias[0])` and `database_exist.alias.push(file_modify)`, now directly inserts into `database_alias` table
- **Problem**: Potential performance difference; original was in-memory move, current implementation involves database I/O which may be slower
- **Solution**: Consider refactoring to optimize performance, possibly by batching inserts or reverting to in-memory operations if appropriate
- **Priority**: Low
- **Status**: Pending